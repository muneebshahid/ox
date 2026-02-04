use crate::{api, tools};
use anyhow::Result;
use futures::StreamExt;
use serde::Deserialize;
use std::io::{self, Write};

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum StreamEvent {
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: serde_json::Value },

    #[serde(rename = "response.output_text.delta")]
    TextDelta { delta: String },

    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: serde_json::Value },

    #[serde(other)]
    Ignored,
}

pub async fn run(
    history: &mut Vec<serde_json::Value>,
    tools_defs: &[serde_json::Value],
    instructions: &str,
) -> Result<()> {
    loop {
        let response = api::call_openai(history, tools_defs, instructions).await?;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut has_tool_calls = false;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // SSE events are separated by double newlines
            while let Some(pos) = buffer.find("\n\n") {
                let event_text = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                // Extract the "data: " line from the SSE event
                let Some(data) = event_text.lines().find_map(|l| l.strip_prefix("data: ")) else {
                    continue;
                };

                let event: StreamEvent = match serde_json::from_str(data) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match event {
                    StreamEvent::OutputItemAdded { item } => {
                        if item["type"] == "function_call" {
                            println!("Calling {}...", item["name"]);
                        }
                    }
                    StreamEvent::TextDelta { delta } => {
                        print!("{delta}");
                        io::stdout().flush()?;
                    }
                    StreamEvent::OutputItemDone { item } => match item["type"].as_str() {
                        Some("message") => {
                            println!();
                            let text = item["content"][0]["text"].as_str().unwrap_or("");
                            history.push(serde_json::json!({
                                "role": "assistant",
                                "content": text
                            }));
                        }
                        Some("function_call") => {
                            let call_id = item["call_id"].as_str().unwrap_or("");
                            let name = item["name"].as_str().unwrap_or("");
                            let arguments = item["arguments"].as_str().unwrap_or("");
                            let result = tools::execute(name, arguments);
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
                            has_tool_calls = true;
                        }
                        _ => {}
                    },
                    StreamEvent::Ignored => {}
                }
            }
        }

        if !has_tool_calls {
            break;
        }
    }
    Ok(())
}
