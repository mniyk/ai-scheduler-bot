// src/main.rs

mod config;
mod domain;
mod error;
mod llm;
mod notification;
mod slack;
mod store;

use std::sync::Arc;

use anyhow::Context;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::llm::OllamaProvider;
use crate::notification::SlackNotifier;
use crate::slack::{run_socket_mode, AppState};
use crate::store::SqlxStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // rustls の暗号化プロバイダを明示的に設定
    // Socket Mode の WebSocket (wss://) で必要
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let config = Config::from_env().context("failed to load configuration")?;

    init_tracing(&config.log_level);
    info!("AI Scheduler Bot starting up");

    let store = SqlxStore::connect(&config.database.url)
        .await
        .context("failed to connect to database")?;
    store.migrate().await.context("failed to run migrations")?;
    info!("database ready");

    let llm = OllamaProvider::new(&config.llm.base_url, &config.llm.model)
        .context("failed to initialize LLM provider")?;
    info!(model = %config.llm.model, "llm provider ready");

    let notifier = SlackNotifier::new();

    let state = AppState {
        store: Arc::new(store),
        llm: Arc::new(llm),
        notifier: Arc::new(notifier),
        slack_bot_token: config.slack.bot_token.clone(),
    };

    info!("starting Slack listener");
    run_socket_mode(state, config.slack.app_token)
        .await
        .context("Slack socket mode terminated")?;

    Ok(())
}

fn init_tracing(default_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}