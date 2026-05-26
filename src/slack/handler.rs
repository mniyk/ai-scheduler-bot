// src/slack/handler.rs

use chrono::Utc;
use tracing::{error, info, warn};

use crate::domain::{Event, NewEvent};
use crate::error::{AppError, AppResult};
use crate::llm::LlmContext;
use crate::notification::NotificationTarget;

use super::response::fetch_user_timezone;
use super::AppState;

pub struct SlashCommandPayload {
    pub team_id: String,
    pub user_id: String,
    pub text: String,
    pub response_url: String,
}

pub struct InteractiveActionPayload {
    pub action_id: String,
    pub value: String,
    pub response_url: String,
}

pub async fn handle_slash_command(state: AppState, payload: SlashCommandPayload) {
    info!(user = %payload.user_id, text = %payload.text, "slash command received");

    let target = NotificationTarget::SlackResponseUrl(payload.response_url.clone());

    // 1. ⏳ 解析中
    if let Err(e) = state.notifier.notify_processing(&target).await {
        warn!(error = %e, "failed to send processing message");
    }

    // 2. 解析 + 保存
    let result = process_schedule(&state, &payload, &target).await;

    // 3. 結果に応じた最終メッセージ
    match result {
        Ok(event) => {
            if let Err(e) = state.notifier.notify_event_created(&target, &event).await {
                warn!(error = %e, "failed to send event_created message");
            }
        }
        Err(AppError::LlmParseFailed) => {
            let _ = state
                .notifier
                .notify_parse_failed(&target, &payload.text)
                .await;
        }
        Err(e) => {
            error!(error = %e, "schedule processing failed");
            let _ = state
                .notifier
                .notify_error(&target, &format!("{}", e))
                .await;
        }
    }
}

async fn process_schedule(
    state: &AppState,
    payload: &SlashCommandPayload,
    target: &NotificationTarget,
) -> AppResult<Event> {
    if payload.text.trim().is_empty() {
        return Err(AppError::Internal(
            "予定の内容を入力してください (例: /schedule 明日15時から1時間、田中さんと打ち合わせ)"
                .into(),
        ));
    }

    let http = reqwest::Client::new();
    let tz = match fetch_user_timezone(&http, &state.slack_bot_token, &payload.user_id).await {
        Ok(tz) => tz,
        Err(e) => {
            warn!(error = %e, user = %payload.user_id, "tz fetch failed; falling back to UTC");
            "UTC".to_string()
        }
    };

    let ctx = LlmContext {
        now: Utc::now(),
        timezone: tz,
    };

    // LLM 解析
    let parsed = state.llm.parse_schedule(&payload.text, &ctx).await?;

    // ✅ 解析完了 (DB保存の前に通知)
    if let Err(e) = state.notifier.notify_parse_completed(target).await {
        warn!(error = %e, "failed to send parse_completed message");
    }

    // ドメイン変換 + DB 保存
    let new_event = NewEvent::from_parsed(
        parsed,
        payload.user_id.clone(),
        payload.team_id.clone(),
    );
    let event = state.store.create(new_event).await?;
    info!(event_id = %event.id, "event created");

    Ok(event)
}

pub async fn handle_interactive_action(state: AppState, payload: InteractiveActionPayload) {
    info!(action = %payload.action_id, value = %payload.value, "interactive action received");

    let target = NotificationTarget::SlackResponseUrl(payload.response_url.clone());

    let result = match payload.action_id.as_str() {
        "event_cancel" => handle_event_cancel(&state, &payload.value, &target).await,
        other => {
            warn!(action = %other, "unknown action_id");
            Ok(())
        }
    };

    if let Err(e) = result {
        error!(error = %e, "interactive action failed");
        let _ = state
            .notifier
            .notify_error(&target, &format!("{}", e))
            .await;
    }
}

async fn handle_event_cancel(
    state: &AppState,
    event_id: &str,
    target: &NotificationTarget,
) -> AppResult<()> {
    // 取消前にイベント情報を取得 (UI更新用)
    let event = state
        .store
        .get(event_id)
        .await?
        .ok_or_else(|| AppError::EventNotFound(event_id.to_string()))?;

    state.store.cancel(event_id).await?;
    info!(event_id = %event_id, "event cancelled");

    // 元メッセージを「取消済み」表示に書き換える
    state.notifier.notify_event_cancelled(target, &event).await?;

    Ok(())
}
