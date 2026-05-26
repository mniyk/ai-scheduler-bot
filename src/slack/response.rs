// src/slack/response.rs

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
struct UsersInfoResponse {
    ok: bool,
    user: Option<UserPayload>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserPayload {
    tz: Option<String>,
}

pub async fn fetch_user_timezone(
    http: &reqwest::Client,
    bot_token: &str,
    user_id: &str,
) -> AppResult<String> {
    let url = format!(
        "https://slack.com/api/users.info?user={}",
        urlencoding::encode(user_id)
    );

    let res = http
        .get(&url)
        .bearer_auth(bot_token)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(AppError::SlackApi(format!(
            "users.info HTTP error: {}",
            res.status()
        )));
    }

    let body: UsersInfoResponse = res.json().await?;
    if !body.ok {
        return Err(AppError::SlackApi(format!(
            "users.info ok=false: {:?}",
            body.error
        )));
    }

    body.user
        .and_then(|u| u.tz)
        .ok_or(AppError::SlackTimezoneUnavailable)
}