// src/llm/ollama.rs

use async_trait::async_trait;
use rig_core::client::{CompletionClient, Nothing};
use rig_core::completion::Prompt;
use rig_core::providers::ollama;

use crate::domain::ParsedSchedule;
use crate::error::{AppError, AppResult};

use super::{prompt, LlmContext, LlmProvider};

pub struct OllamaProvider {
    agent: rig_core::agent::Agent<ollama::CompletionModel>,
}

impl OllamaProvider {
    pub fn new(base_url: &str, model: &str) -> AppResult<Self> {
        let system = prompt::build_system_prompt();

        let client = ollama::Client::builder()
            .api_key(Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| AppError::LlmApi(format!("failed to build Ollama client: {}", e)))?;

        let agent = client.agent(model).preamble(&system).build();

        Ok(Self { agent })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn parse_schedule(
        &self,
        input: &str,
        ctx: &LlmContext,
    ) -> AppResult<ParsedSchedule> {
        let user = prompt::build_user_prompt(input, ctx);

        let response = self
            .agent
            .prompt(user.as_str())
            .await
            .map_err(|e| AppError::LlmApi(e.to_string()))?;

        let json_str = extract_json(&response)
            .ok_or_else(|| AppError::LlmInvalidJson(response.clone()))?;

        let parsed: ParsedSchedule = serde_json::from_str(json_str)
            .map_err(|e| AppError::LlmInvalidJson(format!("{}: {}", e, json_str)))?;

        if parsed.title == "PARSE_FAILED" {
            return Err(AppError::LlmParseFailed);
        }

        Ok(parsed)
    }
}

fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_from_plain() {
        let input = r#"{"title": "x"}"#;
        assert_eq!(extract_json(input), Some(r#"{"title": "x"}"#));
    }

    #[test]
    fn extract_json_with_prefix() {
        let input = r#"以下が結果です:
{"title": "x"}"#;
        assert_eq!(extract_json(input), Some(r#"{"title": "x"}"#));
    }

    #[test]
    fn extract_json_in_markdown() {
        let input = "```json\n{\"title\": \"x\"}\n```";
        assert_eq!(extract_json(input), Some(r#"{"title": "x"}"#));
    }

    #[test]
    fn extract_json_nested() {
        let input = r#"{"a": {"b": 1}, "c": 2}"#;
        assert_eq!(extract_json(input), Some(input));
    }
}