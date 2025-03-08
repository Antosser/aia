use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub openai_token: String,
    pub openai_model: String,
}

impl Config {
    pub fn read(config_path: &Path) -> anyhow::Result<Config> {
        let config_dir = config_path
            .parent()
            .context("Failed to get configuration directory")?;

        if !config_path.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;
            fs::write(&config_path, include_str!("../config_template.conf"))
                .context("Failed to write default config file")?;
        }

        let mut file = std::fs::File::open(&config_path).context("Failed to open config file")?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }
}
