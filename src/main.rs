mod client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payload = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john@abc12.com"
    });
    let url = "https://httpbin.org/post";
    let response = client::post(&payload, None, url).await?;
    println!("Response: {:#?}", response);

    Ok(())
}
