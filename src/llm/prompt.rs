// src/llm/prompt.rs

use super::LlmContext;

/// LLM が出力すべき JSON Schema (人間可読の説明込み)
///
/// Rig や Ollama の structured output 機能で、この Schema に
/// 準拠した出力を強制する。
pub const SCHEDULE_JSON_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["title", "start_at", "end_at", "timezone"],
  "properties": {
    "title":        { "type": "string", "description": "予定のタイトル" },
    "start_at":     { "type": "string", "format": "date-time", "description": "開始日時 (ISO 8601, UTC)" },
    "end_at":       { "type": "string", "format": "date-time", "description": "終了日時 (ISO 8601, UTC)" },
    "timezone":     { "type": "string", "description": "ユーザーのタイムゾーン (例: Asia/Tokyo)" },
    "location":     { "type": ["string", "null"], "description": "物理場所" },
    "meeting_url":  { "type": ["string", "null"], "description": "Web 会議の URL" },
    "related_urls": { "type": "array", "items": { "type": "string" }, "description": "関連 URL の配列" },
    "participants": { "type": "array", "items": { "type": "string" }, "description": "参加者の配列" },
    "notes":        { "type": ["string", "null"], "description": "補足メモ" },
    "reminder_at":  { "type": ["string", "null"], "format": "date-time", "description": "リマインダー時刻 (ISO 8601, UTC)" },
    "recurrence":   { "type": ["string", "null"], "description": "繰り返し設定 (RRULE)" },
    "tag":          { "type": ["string", "null"], "description": "イベント分類タグ" }
  }
}"#;

/// システムプロンプト (LLM の役割定義)
pub const SYSTEM_PROMPT: &str = r#"あなたはスケジュール抽出アシスタントです。
ユーザーが自然言語で書いた予定の文章を、構造化された JSON に変換します。

ルール:
1. 出力は必ず JSON のみ。前後に説明文を付けない。
2. start_at と end_at は ISO 8601 形式の UTC 時刻 (例: "2026-05-27T06:00:00Z")。
3. 与えられた「現在時刻」と「タイムゾーン」を基準に「明日」「来週」などの相対表現を解決する。
4. 終了時刻が明示されていなければ、開始から1時間後をデフォルトとする。
5. 参加者・場所・URL が文章になければ空・null にする。憶測で埋めない。
6. 解析できない場合 (日時が特定できない等) は title フィールドに "PARSE_FAILED" と入れる。
7. title は入力された言語のまま記述する。日本語の入力なら日本語、英語の入力なら英語。翻訳しない。

JSON Schema:
{schema}
"#;

/// ユーザー入力部分のプロンプトを組み立てる
pub fn build_user_prompt(input: &str, ctx: &LlmContext) -> String {
    format!(
        r#"現在時刻 (UTC): {now}
ユーザーのタイムゾーン: {tz}

ユーザー入力:
{input}

上記の入力から予定を抽出し、JSON Schema に準拠した JSON を出力してください。"#,
        now = ctx.now.to_rfc3339(),
        tz = ctx.timezone,
        input = input,
    )
}

/// システムプロンプトに Schema を埋め込んで返す
pub fn build_system_prompt() -> String {
    SYSTEM_PROMPT.replace("{schema}", SCHEDULE_JSON_SCHEMA)
}