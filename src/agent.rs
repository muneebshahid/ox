use anyhow::{Context, Result};

use crate::api::OutputItem;
use crate::tools;

pub async fn run(
    history: &mut Vec<serde_json::Value>,
    tools_defs: &[serde_json::Value],
    instructions: &str,
) -> Result<()> {
    loop {
        let response = crate::api::call_openai(history, tools_defs, instructions).await?;
        let mut has_tool_calls = false;

        for item in &response.output {
            match item {
                OutputItem::Message { content } => {
                    let text = &content.first().context("empty content in response")?.text;
                    println!("{text}");
                    history.push(serde_json::json!({
                        "role": "assistant",
                        "content": text
                    }));
                }
                OutputItem::FunctionCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    has_tool_calls = true;
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
                }
            }
        }

        if !has_tool_calls {
            break;
        }
    }
    Ok(())
}
