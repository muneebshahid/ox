use anyhow::{Context, Result};
use reqwest::Response;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";

pub async fn call_openai(
    client: &reqwest::Client,
    input: &[serde_json::Value],
    tools: &[serde_json::Value],
    instructions: &str,
) -> Result<Response> {
    let key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());
    client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "tools": tools,
            "stream": true
        }))
        .send()
        .await
        .context("failed to send request to OpenAI")
}
