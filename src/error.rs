// src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("LLM API error: {0}")]
    LlmApi(String),

    #[error("LLM returned invalid JSON: {0}")]
    LlmInvalidJson(String),

    #[error("LLM could not parse schedule from input")]
    LlmParseFailed,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Slack API error: {0}")]
    SlackApi(String),

    #[error("Slack user timezone not available")]
    SlackTimezoneUnavailable,

    #[error("Missing environment variable: {0}")]
    ConfigMissing(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;