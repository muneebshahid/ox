use crate::auth;
use anyhow::{Context, Result};
use reqwest::Response;

pub async fn call_openai(
    client: &reqwest::Client,
    input: &[serde_json::Value],
    tools: &[serde_json::Value],
    instructions: &str,
) -> Result<Response> {
    let request_auth = auth::resolve_request_auth(client).await?;
    let model =
        std::env::var("OPENAI_MODEL").unwrap_or_else(|_| request_auth.default_model.to_string());
    let request = client
        .post(request_auth.url)
        .headers(request_auth.headers)
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
