use crate::app_context::AppContext;
use anyhow::{Context, Result};
use reqwest::Response;

pub async fn call_openai(app: &AppContext, history: &[serde_json::Value]) -> Result<Response> {
    let AppContext {
        client,
        auth,
        tool_defs,
        instructions,
    } = app;
    let headers = auth.build_headers(client).await?;
    let request = client
        .post(auth.endpoint())
        .headers(headers)
        .json(&serde_json::json!({
            "model": auth.model(),
            "store": false,
            "instructions": instructions,
            "input": history,
            "tools": tool_defs,
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
