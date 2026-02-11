use crate::app_context::AppContext;
use anyhow::Result;
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
        ..
    } = app;
    let headers = auth.build_headers(client, session_id).await?;

    // Create payload
    let mut payload = json!({
        "model": auth.model(),
        "store": false,
        "instructions": instructions,
        "input": history,
        "tools": tool_defs,
        "stream": true,
        "prompt_cache_key": session_id,
    });
    if let Some(effort) = auth.reasoning_effort() {
        payload["include"] = json!(["reasoning.encrypted_content"]);
        payload["reasoning"] = json!({
            "effort": effort,
            "summary": "auto"
        });
    }

    crate::client::call_with_retry(|| {
        client
            .post(auth.endpoint())
            .headers(headers.clone())
            .json(&payload)
            .send()
    })
    .await
}
