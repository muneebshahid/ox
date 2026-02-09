use crate::app_context::AppContext;
use anyhow::{Context, Result};
use reqwest::Response;
use serde_json::json;

pub async fn call_openai(
    app: &AppContext,
    history: &[serde_json::Value],
    session_id: &str,
) -> Result<Response> {
    let AppContext {
        client,
        auth,
        tool_defs,
        instructions,
    } = app;
    let headers = auth.build_headers(client, session_id).await?;
    let payload = json!({
        "model": auth.model(),
        "store": false,
        "instructions": instructions,
        "input": history,
        "tools": tool_defs,
        "stream": true,
        "include": ["reasoning.encrypted_content"],
        "prompt_cache_key": session_id,
        "reasoning": {
            "effort": auth.reasoning_effort(),
            "summary": "auto"
        }
    });
    let request = client.post(auth.endpoint()).headers(headers).json(&payload);

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
