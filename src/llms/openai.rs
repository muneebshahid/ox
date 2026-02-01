use serde::Deserialize;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";

#[derive(Deserialize)]
pub struct Content {
    pub text: String,
}
#[derive(Deserialize)]
pub struct OpenAIResponse {
    pub content: Option<Vec<Content>>,
}

#[derive(Deserialize)]
pub struct ApiResponse {
    pub output: Vec<OpenAIResponse>,
}

pub async fn call_open_api(prompt: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
    let key = std::env::var("OPENAI_API_KEY")?;
    let client = reqwest::Client::new();
    let res = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", key))
        .json(&serde_json::json!({ "model": "gpt-4",
        "input": prompt }))
        .send()
        .await?;
    Ok(res.json().await?)
}
