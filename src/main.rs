mod llms;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let response = llms::openai::call_open_api("Hello, world!").await.unwrap();
    println!("{}", response.output[0].content.as_ref().unwrap()[0].text);
}
