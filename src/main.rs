mod agent;
mod api;
mod prompt;
mod tools;

use anyhow::Result;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let mut history: Vec<serde_json::Value> = Vec::new();
    let stdin = io::stdin();
    let tool_defs = tools::definitions();
    let instructions = prompt::build();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input.is_empty() {
            break;
        }

        history.push(serde_json::json!({
            "role": "user",
            "content": input
        }));

        agent::run(&mut history, &tool_defs, &instructions).await?;
    }
    Ok(())
}
