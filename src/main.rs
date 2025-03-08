mod config;

use core::panic;
use std::process::exit;

use anyhow::Context;
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use cliclack::{input, intro, log::info, outro, select, spinner};

fn get_config_path() -> anyhow::Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get configuration directory")?
        .join("aia");
    let config_path = config_dir.join("config.toml");

    Ok(config_path)
}

fn get_ai_context() -> anyhow::Result<String> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    let cwd = cwd
        .to_str()
        .context("Failed to get current working directory")?;

    let paths = std::fs::read_dir(cwd).context("Failed to read current working directory")?;

    let binding = paths
        .map(|path| path.context("Failed to get file path"))
        .collect::<Result<Vec<_>, _>>()?;
    let paths = binding
        .iter()
        .map(|path| path.file_name().to_str().unwrap().to_string());

    Ok(format!(
        "Current directory: {}\nFiles in directory: {}",
        cwd,
        paths.collect::<Vec<_>>().join(", ")
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    intro("AIA Terminal Assistant")?;

    let config_path = get_config_path()?;

    let config = config::Config::read(&config_path)?;
    if config.openai_token.is_empty() {
        outro("Please set your OpenAI API key in ~/.config/aia/config.toml")?;
        exit(1);
    }
    unsafe {
        std::env::set_var("OPENAI_API_KEY", config.openai_token);
    }

    let client = Client::new();

    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(include_str!("../system_message.txt"))
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(get_ai_context()?)
            .build()?
            .into(),
    ];

    'infinite: for iteration in 0.. {
        let input = if std::env::args().len() > 1 && iteration == 0 {
            std::env::args().skip(1).collect::<Vec<_>>().join(" ")
        } else {
            input("Input:")
                .interact()
                .context("Failed to parse input")?
        };

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(input.clone())
                .build()?
                .into(),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(&config.openai_model)
            .messages(messages.clone())
            .build()
            .context("Failed to create request")?;

        let spinner = spinner();
        spinner.start("Generating response...");
        let response = client.chat().create(request).await?;
        let response = response.choices[0]
            .message
            .content
            .clone()
            .context("Failed to get response")?
            .replace("```json", "")
            .replace("```", "");
        spinner.stop("Generated response");

        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(response.clone())
                .build()?
                .into(),
        );

        let response = serde_json::from_str::<serde_json::Value>(&response)?;

        match response["type"]
            .as_str()
            .context("Failed to get response type")?
        {
            "command" => {
                let command = response["command"]
                    .as_str()
                    .context("Failed to get command")?;

                info(format!("Command: {}", command))?;

                let selected = select("Pick an action")
                    .item("execute", "Execute", "")
                    .item("follow", "Follow-up", "")
                    .item("quit", "Quit", "")
                    .interact()
                    .context("Failed to parse user selection")?;

                match selected {
                    "execute" => {
                        let output = std::process::Command::new("bash")
                            .arg("-c")
                            .arg(command)
                            .output()
                            .context("Failed to execute command")?;

                        let selected = select("Pick an action")
                            .item("continue", "Continue", "")
                            .item("quit", "Quit", "")
                            .interact()
                            .context("Failed to parse user selection")?;

                        match selected {
                            "continue" => {}
                            "quit" => {
                                break 'infinite;
                            }
                            _ => {
                                panic!("Failed to parse response");
                            }
                        }
                    }
                    "follow" => {
                        messages.push(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content("User did not execute command")
                                .build()?
                                .into(),
                        );
                    }
                    "quit" => {
                        break 'infinite;
                    }
                    _ => {
                        panic!("Failed to parse response");
                    }
                }
            }
            "question" => {
                let question = response["question"]
                    .as_str()
                    .context("Failed to get question")?;

                info(question)?;
            }
            _ => {
                outro("Failed to parse response")?;
                exit(1);
            }
        }
    }

    outro("Goodbye!")?;

    Ok(())
}
