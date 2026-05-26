// src/config.rs

use crate::error::{AppError, AppResult};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub slack: SlackConfig,
    pub llm: LlmConfig,
    pub database: DatabaseConfig,
    pub log_level: String,
}

#[derive(Debug, Clone)]
pub struct SlackConfig {
    pub app_token: String,
    pub bot_token: String,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn from_env() -> AppResult<Self> {
        let _ = dotenvy::dotenv();

        Ok(Self {
            slack: SlackConfig {
                app_token: require_env("SLACK_APP_TOKEN")?,
                bot_token: require_env("SLACK_BOT_TOKEN")?,
            },
            llm: LlmConfig {
                base_url: optional_env("OLLAMA_BASE_URL")
                    .unwrap_or_else(|| "http://localhost:11434".to_string()),
                model: optional_env("OLLAMA_MODEL")
                    .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string()),
            },
            database: DatabaseConfig {
                url: require_env("DATABASE_URL")?,
            },
            log_level: optional_env("RUST_LOG").unwrap_or_else(|| "info".to_string()),
        })
    }
}

fn require_env(key: &str) -> AppResult<String> {
    env::var(key).map_err(|_| AppError::ConfigMissing(key.to_string()))
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok()
}