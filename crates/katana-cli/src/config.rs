use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CliConfig {
    #[serde(default = "default_socket")]
    pub socket: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_format")]
    pub format: OutputFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Table,
    Json,
}

fn default_socket() -> String {
    "/var/run/katana/daemon.sock".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_format() -> OutputFormat {
    OutputFormat::Table
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            socket: default_socket(),
            timeout: default_timeout(),
            format: default_format(),
        }
    }
}

impl CliConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .context(format!("Failed to read config file: {}", config_path.display()))?;
            toml::from_str(&contents)
                .context(format!("Failed to parse config file: {}", config_path.display()))
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create config directory: {}", parent.display()))?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&config_path, contents)
            .context(format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Failed to determine home directory")?;
        Ok(home.join(".katana").join("config.toml"))
    }
}
