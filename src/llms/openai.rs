use serde::Deserialize;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";

#[derive(Deserialize)]
pub struct Content {
    pub text: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OutputItem {
    #[serde(rename = "message")]
    Message { content: Vec<Content> },
    #[serde(rename = "function_call")]
    FunctionCall {
        call_id: String,
        name: String,
        arguments: String,
    },
}

#[derive(Deserialize)]
pub struct ApiResponse {
    pub output: Vec<OutputItem>,
}

pub async fn call_open_api(
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
) -> Result<ApiResponse, Box<dyn std::error::Error>> {
    let key = std::env::var("OPENAI_API_KEY")?;
    let client = reqwest::Client::new();
    let res = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", key))
        .json(&serde_json::json!({ "model": "gpt-4",
        "input": messages, "tools": tools }))
        .send()
        .await?;
    Ok(res.json().await?)
}
