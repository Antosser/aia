mod config;

use std::process::exit;

use anyhow::{Context, Result, anyhow};
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use cliclack::{input, intro, log::info, outro, select, spinner};

fn get_config_path() -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get configuration directory")?
        .join("aia");
    let config_path = config_dir.join("config.toml");
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

    Ok(format!(
        "Current directory: {}\nFiles in directory: {}",
        cwd_str,
        file_names.join(", ")
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    intro("AIA Terminal Assistant").context("Failed to start intro message")?;

    let config_path = get_config_path()?;
    let config = config::Config::read(&config_path).context("Failed to read config file")?;
    if config.openai_token.is_empty() {
        outro("Please set your OpenAI API key in ~/.config/aia/config.toml")
            .context("Failed to display outro message")?;
        exit(1);
    }

    unsafe {
        std::env::set_var("OPENAI_API_KEY", config.openai_token);
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

    let args: Vec<String> = std::env::args().collect();
    'infinite: for iteration in 0.. {
        let input = if args.len() > 1 && iteration == 0 {
            args[1..].join(" ")
        } else {
            input("Input:")
                .interact()
                .context("Failed to parse input")?
        };

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(input.clone())
                .build()
                .context("Failed to build user message")?
                .into(),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(&config.openai_model)
            .messages(messages.clone())
            .build()
            .context("Failed to create request")?;

        let spinner = spinner();
        spinner.start("Generating response...");
        let response = client
            .chat()
            .create(request)
            .await
            .context("Failed to get OpenAI response")?;
        spinner.stop("Generated response");

        let choice = response
            .choices
            .get(0)
            .ok_or_else(|| anyhow!("No choices returned in response"))?;
        let response_content = choice
            .message
            .content
            .clone()
            .ok_or_else(|| anyhow!("Failed to get response content"))?;
        let response_content = response_content.replace("```json", "").replace("```", "");

        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(response_content.clone())
                .build()
                .context("Failed to build assistant message")?
                .into(),
        );

        let response_json = serde_json::from_str::<serde_json::Value>(&response_content)
            .context("Failed to parse response as JSON")?;

        match response_json["type"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get response type"))?
        {
            "command" => {
                let command = response_json["command"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to get command"))?;
                info(format!("Command: {}", command)).context("Failed to log command")?;

                let selected = select("Pick an action")
                    .item("execute", "Execute", "")
                    .item("follow", "Follow-up", "")
                    .item("quit", "Quit", "")
                    .interact()
                    .context("Failed to parse user selection")?;

                match selected {
                    "execute" => {
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

                        if selected == "quit" {
                            break 'infinite;
                        }
                    }
                    "follow" => {
                        messages.push(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content("User did not execute command")
                                .build()
                                .context("Failed to build follow-up message")?
                                .into(),
                        );
                    }
                    "quit" => break 'infinite,
                    _ => return Err(anyhow!("Invalid selection")),
                }
            }
            "question" => {
                let question = response_json["question"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to get question"))?;
                info(question).context("Failed to log question")?;
            }
            _ => return Err(anyhow!("Unexpected response type")),
        }
    }

    outro("Goodbye!").context("Failed to display outro message")?;
    Ok(())
}
