mod llms;
use llms::openai::Message;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let mut history: Vec<Message> = Vec::new();
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input.is_empty() {
            break;
        }

        history.push(Message {
            role: "user".to_string(),
            content: input.to_string(),
        });

        let response = llms::openai::call_open_api(&history).await?;

        let text = response.output[0].content.as_ref().unwrap()[0].text.clone();
        println!("{}", text);

        history.push(Message {
            role: "assistant".to_string(),
            content: text,
        });
    }
    Ok(())
}
