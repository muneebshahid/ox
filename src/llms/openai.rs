use serde::Deserialize;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";

//     {
//         "id": "msg_67b73f697ba4819183a15cc17d011509",
//         "type": "message",
//         "role": "assistant",
//         "content": [
//             {
//                 "type": "output_text",
//                 "text": "Under the soft glow of the moon, Luna the unicorn danced through fields of twinkling stardust, leaving trails of dreams for every child asleep.",
//                 "annotations": []
//             }
//         ]
//     }
///

#[derive(Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    content_type: String,
    pub text: String,
    annotations: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
pub struct OpenAIResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
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
