mod config;

use std::{io::Read, process::exit};

use anyhow::{Context, Result, anyhow};
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use cliclack::{input, intro, outro, select, spinner};
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create the /var/log/aia directory if it doesn't exist
    let log_dir = "/var/log/aia";
    if !std::path::Path::new(log_dir).exists() {
        std::fs::create_dir_all(log_dir)?;
        debug!("Created log directory: {}", log_dir);
    }

    // Create a file appender for the log file
    let file_appender = tracing_appender::rolling::daily(log_dir, "aia.log");

    // Set up the tracing subscriber
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry().with(file_layer).init();

    info!("Logging initialized successfully");
    Ok(())
}

fn get_config_path() -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get configuration directory")?
        .join("aia");
    let config_path = config_dir.join("config.toml");
    debug!("Config path: {:?}", config_path);
    Ok(config_path)
}

fn get_ai_context() -> Result<String> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    let cwd_str = cwd
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert current working directory to string"))?;

    let paths = std::fs::read_dir(&cwd).context("Failed to read current working directory")?;
    let file_names = paths
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    let context = format!(
        "Current directory: {}\nFiles in directory: {}",
        cwd_str,
        file_names.join(", ")
    );
    debug!("AI context: {}", context);
    Ok(context)
}

fn get_piped_input() -> anyhow::Result<Option<String>> {
    if atty::is(atty::Stream::Stdin) {
        debug!("No piped input detected");
        return Ok(None);
    }

    let mut buffer = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buffer)
        .context("Failed to read piped input")?;
    debug!("Piped input: {}", buffer);
    Ok(Some(buffer))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging().unwrap();
    info!("Starting AIA Terminal Assistant");

    intro("AIA Terminal Assistant").context("Failed to start intro message")?;
    info!("Intro message displayed");

    let config_path = get_config_path()?;
    let config = config::Config::read(&config_path).context("Failed to read config file")?;
    debug!("Config loaded: {:?}", config);

    if config.openai_token.is_empty() {
        error!("OpenAI API key is not set in config");
        outro("Please set your OpenAI API key in ~/.config/aia/config.toml")
            .context("Failed to display outro message")?;
        exit(1);
    }

    unsafe {
        std::env::set_var("OPENAI_API_KEY", config.openai_token);
        debug!("OpenAI API key set in environment");
    }

    let client = Client::new();
    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(include_str!("../system_message.txt"))
            .build()
            .context("Failed to build system message")?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(get_ai_context()?)
            .build()
            .context("Failed to build AI context message")?
            .into(),
    ];
    debug!("Initial messages: {:?}", messages);

    if let Some(piped_input) = get_piped_input().context("Failed to get piped input")? {
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(piped_input)
                .build()
                .context("Failed to build piped input message")?
                .into(),
        );
        debug!("Added piped input to messages");
    }

    let args: Vec<String> = std::env::args().collect();
    debug!("Command-line arguments: {:?}", args);

    'infinite: for iteration in 0.. {
        let input = if args.len() > 1 && iteration == 0 {
            args[1..].join(" ")
        } else {
            input("Input:")
                .interact()
                .context("Failed to parse input")?
        };
        debug!("User input: {}", input);

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(input.clone())
                .build()
                .context("Failed to build user message")?
                .into(),
        );
        debug!("Updated messages: {:?}", messages);

        {
            let request = CreateChatCompletionRequestArgs::default()
                .model(&config.openai_model)
                .messages(messages.clone())
                .build()
                .context("Failed to create request")?;
            debug!("Request created: {:?}", request);

            let spinner = spinner();
            spinner.start("Thinking...");
            info!("Thinking...");
            let response = client
                .chat()
                .create(request)
                .await
                .context("Failed to get OpenAI response")?;
            spinner.stop("Thought");
            info!("Received response from OpenAI");

            let choice = response
                .choices
                .first()
                .ok_or_else(|| anyhow!("No choices returned in response"))?;
            let response_content = choice
                .message
                .content
                .clone()
                .ok_or_else(|| anyhow!("Failed to get response content"))?;
            debug!("Response content: {}", response_content);

            messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(response_content.clone())
                    .build()
                    .context("Failed to build assistant message")?
                    .into(),
            );
            debug!("Updated messages with assistant response");
        }

        let request = CreateChatCompletionRequestArgs::default()
            .model(&config.openai_model)
            .messages(messages.clone())
            .build()
            .context("Failed to create request")?;
        debug!("Request created: {:?}", request);

        let spinner = spinner();
        spinner.start("Generating response...");
        info!("Generating response...");
        let response = client
            .chat()
            .create(request)
            .await
            .context("Failed to get OpenAI response")?;
        spinner.stop("Generated response");
        info!("Generated response");

        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices returned in response"))?;
        let response_content = choice
            .message
            .content
            .clone()
            .ok_or_else(|| anyhow!("Failed to get response content"))?;
        let response_content = response_content.replace("```json", "").replace("```", "");
        debug!("Response content: {}", response_content);

        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(response_content.clone())
                .build()
                .context("Failed to build assistant message")?
                .into(),
        );
        debug!("Updated messages with final assistant response");

        let response_json = serde_json::from_str::<serde_json::Value>(&response_content)
            .context("Failed to parse response as JSON")?;
        debug!("Parsed response JSON: {:?}", response_json);

        match response_json["type"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get response type"))?
        {
            "command" => {
                let command = response_json["command"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to get command"))?;
                info!(command = command, "Command received");

                let selected = select("Pick an action")
                    .item("execute", "Execute", "")
                    .item("follow", "Follow-up", "")
                    .item("quit", "Quit", "")
                    .interact()
                    .context("Failed to parse user selection")?;
                debug!("User selected: {}", selected);

                match selected {
                    "execute" => {
                        info!("Executing command: {}", command);
                        std::process::Command::new("bash")
                            .arg("-c")
                            .arg(command)
                            .output()
                            .context("Failed to execute command")?;

                        let selected = select("Pick an action")
                            .item("continue", "Continue", "")
                            .item("quit", "Quit", "")
                            .interact()
                            .context("Failed to parse user selection")?;
                        debug!("User selected: {}", selected);

                        if selected == "quit" {
                            info!("User chose to quit");
                            break 'infinite;
                        }
                    }
                    "follow" => {
                        info!("User chose to follow up");
                        messages.push(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content("User did not execute command")
                                .build()
                                .context("Failed to build follow-up message")?
                                .into(),
                        );
                    }
                    "quit" => {
                        info!("User chose to quit");
                        break 'infinite;
                    }
                    _ => return Err(anyhow!("Invalid selection")),
                }
            }
            "question" => {
                let question = response_json["question"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to get question"))?;
                info!(question = question, "Question received");
            }
            "answer" => {
                let answer = response_json["answer"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to get answer"))?;
                info!(answer = answer, "Answer received");
            }
            _ => {
                error!("Unexpected response type");
                return Err(anyhow!("Unexpected response type"));
            }
        }
    }

    info!("Exiting AIA Terminal Assistant");
    outro("Goodbye!").context("Failed to display outro message")?;

    Ok(())
}
