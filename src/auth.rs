use anyhow::{Context, Result};
use base64::Engine;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use std::path::PathBuf;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";
const SUBSCRIPTION_API_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const JWT_CLAIM_PATH: &str = "https://api.openai.com/auth";

#[derive(Clone, Copy)]
enum AuthMode {
    ApiKey,
    Subscription,
}

#[derive(Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexTokens>,
}

#[derive(Deserialize)]
struct CodexTokens {
    access_token: String,
    account_id: Option<String>,
}

struct SubscriptionAuth {
    access_token: String,
    account_id: String,
}

fn auth_mode() -> AuthMode {
    let raw = std::env::var("AUTH_MODE").unwrap_or_else(|_| "api".to_string());
    match raw.to_ascii_lowercase().as_str() {
        "subscription" => AuthMode::Subscription,
        _ => AuthMode::ApiKey,
    }
}

pub fn request_url() -> &'static str {
    match auth_mode() {
        AuthMode::ApiKey => OPENAI_API_URL,
        AuthMode::Subscription => SUBSCRIPTION_API_URL,
    }
}

pub fn default_model() -> &'static str {
    match auth_mode() {
        AuthMode::ApiKey => "gpt-4.1-mini",
        AuthMode::Subscription => "gpt-5.3-codex",
    }
}

fn codex_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn extract_account_id_from_jwt(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let _sig = parts.next()?;

    let payload = payload.replace('-', "+").replace('_', "/");
    let padded = match payload.len() % 4 {
        2 => format!("{payload}=="),
        3 => format!("{payload}="),
        _ => payload,
    };

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(padded)
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get(JWT_CLAIM_PATH)?
        .get("chatgpt_account_id")?
        .as_str()
        .map(std::string::ToString::to_string)
}

fn load_subscription_auth() -> Result<SubscriptionAuth> {
    let auth_path = codex_home()?.join("auth.json");
    let raw = std::fs::read_to_string(&auth_path)
        .with_context(|| format!("failed to read {}", auth_path.display()))?;
    let auth: CodexAuthFile = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON in {}", auth_path.display()))?;

    let tokens = auth
        .tokens
        .context("missing tokens in Codex auth file; run `codex login`")?;
    let account_id = tokens
        .account_id
        .or_else(|| extract_account_id_from_jwt(&tokens.access_token))
        .context("missing account_id in Codex auth tokens; run `codex login`")?;

    Ok(SubscriptionAuth {
        access_token: tokens.access_token,
        account_id,
    })
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<()> {
    let key = HeaderName::from_static(name);
    let val = HeaderValue::from_str(value)
        .with_context(|| format!("invalid header value for '{}'", key.as_str()))?;
    headers.insert(key, val);
    Ok(())
}

pub fn get_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    match auth_mode() {
        AuthMode::ApiKey => {
            let key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
            insert_header(&mut headers, "authorization", &format!("Bearer {key}"))?;
        }
        AuthMode::Subscription => {
            let auth = load_subscription_auth()?;
            insert_header(
                &mut headers,
                AUTHORIZATION.as_str(),
                &format!("Bearer {}", auth.access_token),
            )?;
            insert_header(&mut headers, "chatgpt-account-id", &auth.account_id)?;
            headers.insert(
                HeaderName::from_static("openai-beta"),
                HeaderValue::from_static("responses=experimental"),
            );
            headers.insert(
                HeaderName::from_static("accept"),
                HeaderValue::from_static("text/event-stream"),
            );
            headers.insert(
                HeaderName::from_static("originator"),
                HeaderValue::from_static("ox"),
            );
        }
    }

    Ok(headers)
}
