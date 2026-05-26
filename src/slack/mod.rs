pub mod handler;
pub mod response;
pub mod socket;

use std::sync::Arc;

use crate::llm::LlmProvider;
use crate::notification::NotificationChannel;
use crate::store::EventStore;

pub use socket::run_socket_mode;

/// Slack ハンドラに渡す共有状態
///
/// 各依存をトレイトオブジェクトで持ち、テスト時にモックに差し替え可能。
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn EventStore>,
    pub llm: Arc<dyn LlmProvider>,
    pub notifier: Arc<dyn NotificationChannel>,
    pub slack_bot_token: String,
}