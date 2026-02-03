mod llms;
use crate::llms::openai::OutputItem;
use std::io::{self, BufRead, Write};

fn get_tools() -> Vec<serde_json::Value> {
    let tools = vec![serde_json::json!({
    "type": "function",
    "name": "read_file",
    "description": "Read the contents of a file at the given path",
    "parameters": {
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "The file path to read" }
        },
        "required": ["path"]
    }})];
    tools
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let mut history: Vec<serde_json::Value> = Vec::new();
    let stdin = io::stdin();
    let tools = get_tools();

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
        loop {
            let response = llms::openai::call_open_api(&history, &tools).await?;
            let mut has_tool_calls = false;
            for item in &response.output {
                match item {
                    OutputItem::Message { content } => {
                        println!("{}", content[0].text);
                        history.push(serde_json::json!({
                            "role": "assistant",
                            "content": content[0].text
                        }));
                    }
                    OutputItem::FunctionCall {
                        call_id,
                        name,
                        arguments,
                    } => {
                        has_tool_calls = true;
                        let args: serde_json::Value = serde_json::from_str(arguments)?;

                        let result = match name.as_str() {
                            "read_file" => {
                                let path = args["path"].as_str().ok_or("missing path")?;
                                std::fs::read_to_string(path)
                                    .unwrap_or_else(|e| format!("Error: {}", e))
                            }
                            _ => format!("Unknown tool: {}", name),
                        };
                        history.push(serde_json::json!({
                            "type": "function_call",
                            "call_id": call_id,
                            "name": name,
                            "arguments": arguments
                        }));
                        history.push(serde_json::json!({
                            "type": "function_call_output",
                            "call_id": call_id,
                            "output": result
                        }));
                    }
                }
            }
            if !has_tool_calls {
                break;
            }
        }
    }
    Ok(())
}
