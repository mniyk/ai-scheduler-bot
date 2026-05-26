pub mod slack;

use async_trait::async_trait;

use crate::domain::Event;
use crate::error::AppResult;

pub use slack::SlackNotifier;

#[async_trait]
pub trait NotificationChannel: Send + Sync {
    async fn notify_processing(&self, target: &NotificationTarget) -> AppResult<()>;
    async fn notify_parse_completed(&self, target: &NotificationTarget) -> AppResult<()>;
    async fn notify_event_created(
        &self,
        target: &NotificationTarget,
        event: &Event,
    ) -> AppResult<()>;
    /// イベント取消後、元のメッセージを「取消済み」表示に書き換える
    async fn notify_event_cancelled(
        &self,
        target: &NotificationTarget,
        event: &Event,
    ) -> AppResult<()>;
    async fn notify_parse_failed(
        &self,
        target: &NotificationTarget,
        original_input: &str,
    ) -> AppResult<()>;
    async fn notify_error(
        &self,
        target: &NotificationTarget,
        message: &str,
    ) -> AppResult<()>;
}

#[derive(Debug, Clone)]
pub enum NotificationTarget {
    SlackResponseUrl(String),
}