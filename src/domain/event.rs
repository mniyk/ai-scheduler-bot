use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// LLM が自然言語から抽出した構造化スケジュール
///
/// JSON Schema に沿って LLM が出力する形。
/// この時点では DB 文脈 (user_id 等) を持たない。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSchedule {
    pub title: String,

    /// 開始日時 (UTC)
    pub start_at: DateTime<Utc>,

    /// 終了日時 (UTC)
    pub end_at: DateTime<Utc>,

    /// タイムゾーン文字列 ("Asia/Tokyo" 等)
    pub timezone: String,

    /// 物理場所 (オフィス、会議室など)
    #[serde(default)]
    pub location: Option<String>,

    /// Web 会議 URL
    #[serde(default)]
    pub meeting_url: Option<String>,

    /// 関連 URL (アジェンダ、資料など)
    #[serde(default)]
    pub related_urls: Vec<String>,

    /// 参加者
    #[serde(default)]
    pub participants: Vec<String>,

    /// 補足メモ
    #[serde(default)]
    pub notes: Option<String>,

    /// リマインダー時刻 (UTC)
    #[serde(default)]
    pub reminder_at: Option<DateTime<Utc>>,

    /// 繰り返し設定 (RRULE 形式の文字列)
    #[serde(default)]
    pub recurrence: Option<String>,

    /// タグ (分類用)
    #[serde(default)]
    pub tag: Option<String>,
}

/// DB に保存する直前のイベント
///
/// `ParsedSchedule` に Slack 文脈 (user_id, team_id) を付与したもの。
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub id: String,
    pub slack_user_id: String,
    pub slack_team_id: String,
    pub schedule: ParsedSchedule,
}

impl NewEvent {
    /// LLM 出力 + Slack 文脈から新規イベントを組み立てる
    pub fn from_parsed(
        schedule: ParsedSchedule,
        slack_user_id: String,
        slack_team_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            slack_user_id,
            slack_team_id,
            schedule,
        }
    }
}

/// DB から取得した完全なイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub slack_user_id: String,
    pub slack_team_id: String,
    pub title: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub timezone: String,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub related_urls: Vec<String>,
    pub participants: Vec<String>,
    pub notes: Option<String>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
    pub tag: Option<String>,
    pub status: EventStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// イベントのステータス (論理削除を含む)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Active,
    Cancelled,
    Done,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Active => "active",
            EventStatus::Cancelled => "cancelled",
            EventStatus::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(EventStatus::Active),
            "cancelled" => Some(EventStatus::Cancelled),
            "done" => Some(EventStatus::Done),
            _ => None,
        }
    }
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Active
    }
}
