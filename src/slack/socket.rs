// src/slack/socket.rs

use std::sync::Arc;

use slack_morphism::prelude::*;
use tracing::info;

use crate::error::AppResult;

use super::handler::{
    handle_interactive_action, handle_slash_command,
    InteractiveActionPayload, SlashCommandPayload,
};
use super::AppState;

pub async fn run_socket_mode(state: AppState, app_token: String) -> AppResult<()> {
    info!("starting Slack Socket Mode listener");

    let client = Arc::new(SlackClient::new(
        SlackClientHyperConnector::new()
            .map_err(|e| crate::error::AppError::SlackApi(e.to_string()))?,
    ));

    let app_token_value: SlackApiTokenValue = app_token.into();
    let app_token = SlackApiToken::new(app_token_value);

    let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
        .with_command_events(on_command_event)
        .with_interaction_events(on_interaction_event);

    let listener_environment = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone())
            .with_user_state(state),
    );

    let socket_mode_listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_environment.clone(),
        socket_mode_callbacks,
    );

    socket_mode_listener.listen_for(&app_token).await
        .map_err(|e| crate::error::AppError::SlackApi(e.to_string()))?;

    socket_mode_listener.serve().await;

    Ok(())
}

async fn on_command_event(
    event: SlackCommandEvent,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> Result<SlackCommandEventResponse, Box<dyn std::error::Error + Send + Sync>> {
    let state = states
        .read()
        .await
        .get_user_state::<AppState>()
        .cloned()
        .expect("AppState must be set");

    if event.command.0 != "/schedule" {
        return Ok(SlackCommandEventResponse::new(
            SlackMessageContent::new().with_text("unsupported command".into()),
        ));
    }

    let payload = SlashCommandPayload {
        team_id: event.team_id.to_string(),
        user_id: event.user_id.to_string(),
        text: event.text.unwrap_or_default(),
        response_url: event.response_url.0.to_string(),
    };

    tokio::spawn(async move {
        handle_slash_command(state, payload).await;
    });

    Ok(SlackCommandEventResponse::new(
        SlackMessageContent::new(),
    ))
}

async fn on_interaction_event(
    event: SlackInteractionEvent,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = states
        .read()
        .await
        .get_user_state::<AppState>()
        .cloned()
        .expect("AppState must be set");

    let block_actions = match event {
        SlackInteractionEvent::BlockActions(b) => b,
        _ => return Ok(()),
    };

    let response_url = block_actions
        .response_url
        .as_ref()
        .map(|u| u.0.to_string())
        .unwrap_or_default();

    if let Some(actions) = block_actions.actions {
        for action in actions {
            let payload = InteractiveActionPayload {
                action_id: action.action_id.0.clone(),
                value: action.value.unwrap_or_default(),
                response_url: response_url.clone(),
            };

            let state_cloned = state.clone();
            tokio::spawn(async move {
                handle_interactive_action(state_cloned, payload).await;
            });
        }
    }

    Ok(())
}