pub mod ollama;
pub mod prompt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::ParsedSchedule;
use crate::error::AppResult;

pub use ollama::OllamaProvider;

/// LLM 呼び出しのコンテキスト
///
/// 自然言語の解釈に必要な「現在時刻」「ユーザーのタイムゾーン」を
/// 呼び出し側から渡せるようにしている。
#[derive(Debug, Clone)]
pub struct LlmContext {
    /// 現在時刻 (UTC)。LLM は現在時刻を知らないので必ず渡す。
    pub now: DateTime<Utc>,

    /// ユーザーのタイムゾーン (例: "Asia/Tokyo")
    pub timezone: String,
}

/// LLM プロバイダの抽象インターフェース
///
/// 実装は `OllamaProvider` のみだが、
/// 将来 Anthropic / OpenAI を追加する余地としてトレイトで抽象化。
/// テスト時にはモック実装を差し込める。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 自然言語の文章を `ParsedSchedule` に変換する
    async fn parse_schedule(
        &self,
        input: &str,
        ctx: &LlmContext,
    ) -> AppResult<ParsedSchedule>;
}