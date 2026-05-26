// src/notification/slack.rs

use async_trait::async_trait;
use chrono::TimeZone;
use chrono_tz::Tz;
use serde_json::{json, Value};

use crate::domain::Event;
use crate::error::{AppError, AppResult};

use super::{NotificationChannel, NotificationTarget};

pub struct SlackNotifier {
    http: reqwest::Client,
}

impl SlackNotifier {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    async fn post_to_response_url(&self, url: &str, payload: Value) -> AppResult<()> {
        let res = self.http.post(url).json(&payload).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::SlackApi(format!(
                "response_url POST failed: {} {}",
                status, body
            )));
        }
        Ok(())
    }

    async fn send(&self, target: &NotificationTarget, payload: Value) -> AppResult<()> {
        match target {
            NotificationTarget::SlackResponseUrl(url) => {
                self.post_to_response_url(url, payload).await
            }
        }
    }
}

#[async_trait]
impl NotificationChannel for SlackNotifier {
    async fn notify_processing(&self, target: &NotificationTarget) -> AppResult<()> {
        let payload = json!({
            "response_type": "ephemeral",
            "text": "⏳ 解析中..."
        });
        self.send(target, payload).await
    }

    async fn notify_parse_completed(&self, target: &NotificationTarget) -> AppResult<()> {
        let payload = json!({
            "response_type": "ephemeral",
            "text": "✅ 解析完了"
        });
        self.send(target, payload).await
    }

    async fn notify_event_created(
        &self,
        target: &NotificationTarget,
        event: &Event,
    ) -> AppResult<()> {
        let payload = json!({
            "response_type": "in_channel",
            "text": format!("✅ 予定を登録しました: {}", event.title),
            "blocks": build_event_blocks(event),
        });
        self.send(target, payload).await
    }

    async fn notify_parse_failed(
        &self,
        target: &NotificationTarget,
        original_input: &str,
    ) -> AppResult<()> {
        let payload = json!({
            "response_type": "ephemeral",
            "text": format!(
                "❌ 予定を解析できませんでした。\n\
                 もう少し具体的に書いてください (例: 「明日15時から1時間、田中さんと打ち合わせ」)。\n\
                 入力: 「{}」",
                original_input
            ),
        });
        self.send(target, payload).await
    }

    async fn notify_event_cancelled(
        &self,
        target: &NotificationTarget,
        event: &Event,
    ) -> AppResult<()> {
        let payload = json!({
            "response_type": "in_channel",
            "replace_original": true,
            "text": format!("❌ 取消しました: {}", event.title),
            "blocks": build_cancelled_event_blocks(event),
        });
        self.send(target, payload).await
    }

    async fn notify_error(
        &self,
        target: &NotificationTarget,
        message: &str,
    ) -> AppResult<()> {
        let payload = json!({
            "response_type": "ephemeral",
            "text": format!("⚠️ エラーが発生しました: {}", message),
        });
        self.send(target, payload).await
    }
}

fn build_event_blocks(event: &Event) -> Value {
    let tz: Tz = event.timezone.parse().unwrap_or(chrono_tz::UTC);
    let start_local = tz.from_utc_datetime(&event.start_at.naive_utc());
    let end_local = tz.from_utc_datetime(&event.end_at.naive_utc());

    let mut fields: Vec<Value> = Vec::new();

    fields.push(json!({
        "type": "mrkdwn",
        "text": format!("*開始:*\n{}", start_local.format("%Y-%m-%d %H:%M %Z"))
    }));
    fields.push(json!({
        "type": "mrkdwn",
        "text": format!("*終了:*\n{}", end_local.format("%Y-%m-%d %H:%M %Z"))
    }));

    if let Some(loc) = &event.location {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*場所:*\n{}", loc) }));
    }
    if let Some(url) = &event.meeting_url {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*会議URL:*\n<{}>", url) }));
    }
    if !event.participants.is_empty() {
        fields.push(json!({
            "type": "mrkdwn",
            "text": format!("*参加者:*\n{}", event.participants.join(", "))
        }));
    }
    if let Some(tag) = &event.tag {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*タグ:*\n{}", tag) }));
    }

    let mut blocks = vec![
        json!({
            "type": "header",
            "text": { "type": "plain_text", "text": format!("✅ {}", event.title) }
        }),
        json!({
            "type": "section",
            "fields": fields
        }),
    ];

    if !event.related_urls.is_empty() {
        let urls = event
            .related_urls
            .iter()
            .map(|u| format!("• <{}>", u))
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("*関連URL:*\n{}", urls) }
        }));
    }

    if let Some(notes) = &event.notes {
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("*メモ:*\n{}", notes) }
        }));
    }

    blocks.push(json!({
        "type": "actions",
        "elements": [
            {
                "type": "button",
                "style": "danger",
                "text": { "type": "plain_text", "text": "❌ 取消" },
                "action_id": "event_cancel",
                "value": event.id,
            }
        ]
    }));

    json!(blocks)
}

fn build_cancelled_event_blocks(event: &Event) -> Value {
    let tz: Tz = event.timezone.parse().unwrap_or(chrono_tz::UTC);
    let start_local = tz.from_utc_datetime(&event.start_at.naive_utc());
    let end_local = tz.from_utc_datetime(&event.end_at.naive_utc());

    let mut fields: Vec<Value> = Vec::new();

    fields.push(json!({
        "type": "mrkdwn",
        "text": format!("*開始:*\n{}", start_local.format("%Y-%m-%d %H:%M %Z"))
    }));
    fields.push(json!({
        "type": "mrkdwn",
        "text": format!("*終了:*\n{}", end_local.format("%Y-%m-%d %H:%M %Z"))
    }));

    if let Some(loc) = &event.location {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*場所:*\n{}", loc) }));
    }
    if let Some(url) = &event.meeting_url {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*会議URL:*\n<{}>", url) }));
    }
    if !event.participants.is_empty() {
        fields.push(json!({
            "type": "mrkdwn",
            "text": format!("*参加者:*\n{}", event.participants.join(", "))
        }));
    }
    if let Some(tag) = &event.tag {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*タグ:*\n{}", tag) }));
    }

    let blocks = vec![
        json!({
            "type": "header",
            "text": { "type": "plain_text", "text": format!("❌ [取消済み] {}", event.title) }
        }),
        json!({
            "type": "section",
            "fields": fields
        }),
        json!({
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": "_この予定は取消されました_"
                }
            ]
        })
    ];

    json!(blocks)
}