use crate::auth;
use anyhow::{Context, Result};
use reqwest::Response;

pub async fn call_openai(
    client: &reqwest::Client,
    input: &[serde_json::Value],
    tools: &[serde_json::Value],
    instructions: &str,
) -> Result<Response> {
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| auth::default_model().to_string());
    let request = client
        .post(auth::request_url())
        .headers(auth::get_headers()?)
        .json(&serde_json::json!({
            "model": model,
            "store": false,
            "instructions": instructions,
            "input": input,
            "tools": tools,
            "stream": true
        }));

    let response = request
        .send()
        .await
        .context("failed to send request to OpenAI")?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(response),
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "OpenAI API returned error status: ({status}): {body}"
            ))
        }
    }
}
