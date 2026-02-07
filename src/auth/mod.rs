mod config;
mod store;
mod subscriptions;

use anyhow::{Context, Result};
use config::AuthMode;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};

pub use config::AuthConfig;

struct SubscriptionAuth {
    access_token: String,
    account_id: String,
}

impl AuthConfig {
    pub async fn build_headers(&self, client: &reqwest::Client) -> Result<HeaderMap> {
        match self.mode() {
            AuthMode::ApiKey => api_key_headers(),
            AuthMode::Subscription => build_subscription_headers(client).await,
        }
    }
}

async fn get_valid_tokens(
    client: &reqwest::Client,
    tokens: &mut store::CodexTokens,
) -> Result<bool> {
    let access_token = tokens
        .access_token
        .as_deref()
        .context("missing access_token in Codex auth tokens; run `codex login`")?;

    if !subscriptions::is_access_token_expired(access_token) {
        return Ok(false);
    }

    let refresh_token = tokens
        .refresh_token
        .as_deref()
        .context("missing refresh_token in Codex auth tokens; run `codex login`")?;

    let refreshed = subscriptions::refresh_access_token(client, refresh_token).await?;
    tokens.access_token = Some(refreshed.access_token.clone());

    if let Some(next_refresh_token) = refreshed.refresh_token {
        tokens.refresh_token = Some(next_refresh_token);
    }
    if let Some(next_id_token) = refreshed.id_token {
        tokens.id_token = Some(next_id_token);
    }

    tokens.account_id = refreshed
        .account_id
        .or_else(|| subscriptions::extract_account_id_from_jwt(&refreshed.access_token))
        .or_else(|| tokens.account_id.clone());

    Ok(true)
}

async fn load_subscription_auth(client: &reqwest::Client) -> Result<SubscriptionAuth> {
    let mut auth_file = store::load_auth_file()?;
    let mut tokens = auth_file
        .tokens
        .take()
        .context("missing tokens in Codex auth file; run `codex login`")?;

    let did_refresh = get_valid_tokens(client, &mut tokens).await?;
    if did_refresh {
        auth_file.tokens = Some(tokens.clone());
        store::save_auth_file(&auth_file)?;
    }

    let access_token = tokens
        .access_token
        .as_deref()
        .context("missing access_token in Codex auth tokens; run `codex login`")?;
    let account_id = tokens
        .account_id
        .clone()
        .or_else(|| subscriptions::extract_account_id_from_jwt(access_token))
        .context("missing account_id in Codex auth tokens; run `codex login`")?;

    Ok(SubscriptionAuth {
        access_token: access_token.to_string(),
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

fn api_key_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    let key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    insert_header(
        &mut headers,
        AUTHORIZATION.as_str(),
        &format!("Bearer {key}"),
    )?;
    Ok(headers)
}

async fn build_subscription_headers(client: &reqwest::Client) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    let auth = load_subscription_auth(client).await?;
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
    Ok(headers)
}
