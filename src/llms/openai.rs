const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";

pub async fn call_open_api(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let key = std::env::var("OPENAI_API_KEY")?;
    let client = reqwest::Client::new();
    let res = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", key))
        .json(&serde_json::json!({ "prompt": prompt }))
        .send()
        .await?;
    Ok(res.text().await?)
}
