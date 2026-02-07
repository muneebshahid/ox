use anyhow::{Context, Result, anyhow};
use jsonwebtoken::dangerous::insecure_decode;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CLOCK_SKEW_SECONDS: i64 = 60;

#[derive(Debug)]
pub(super) struct RefreshedSubscriptionTokens {
    pub(super) access_token: String,
    pub(super) refresh_token: Option<String>,
    pub(super) id_token: Option<String>,
    pub(super) account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    exp: Option<i64>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    auth: Option<JwtAuthClaims>,
}

#[derive(Debug, Deserialize)]
struct JwtAuthClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    #[serde(rename = "access_token")]
    access: Option<String>,
    #[serde(rename = "refresh_token")]
    refresh: Option<String>,
    #[serde(rename = "id_token")]
    id: Option<String>,
}

fn decode_jwt_claims(token: &str) -> Option<JwtClaims> {
    // We intentionally only decode claims for local expiry/account checks.
    // Token authenticity is enforced by OpenAI when the token is used.
    insecure_decode::<JwtClaims>(token)
        .ok()
        .map(|token_data| token_data.claims)
}

pub(super) fn extract_account_id_from_jwt(token: &str) -> Option<String> {
    decode_jwt_claims(token)?.auth?.chatgpt_account_id
}

pub(super) fn is_access_token_expired(token: &str) -> bool {
    let Some(claims) = decode_jwt_claims(token) else {
        return true;
    };
    let Some(exp) = claims.exp else {
        return true;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_i64, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    now + CLOCK_SKEW_SECONDS >= exp
}

pub(super) async fn refresh_access_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<RefreshedSubscriptionTokens> {
    let response = client
        .post(TOKEN_URL)
        .header("content-type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ])
        .send()
        .await
        .context("failed to send token refresh request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "subscription token refresh failed ({status}): {body}"
        ));
    }

    let payload: RefreshResponse = response
        .json()
        .await
        .context("failed to parse token refresh response")?;
    let access_token = payload
        .access
        .context("token refresh response missing access_token")?;
    let account_id = extract_account_id_from_jwt(&access_token);

    Ok(RefreshedSubscriptionTokens {
        access_token,
        refresh_token: payload.refresh,
        id_token: payload.id,
        account_id,
    })
}
