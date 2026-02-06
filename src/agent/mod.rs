mod events;
mod handler;
mod stream;
#[cfg(test)]
mod stream_tests;

use crate::api;
use anyhow::Result;
use futures::StreamExt;
use handler::EventHandler;
use stream::{get_event, parse_event};

const MAX_TOOL_CALLS: usize = 20;

pub async fn run(
    client: &reqwest::Client,
    history: &mut Vec<serde_json::Value>,
    tools_defs: &[serde_json::Value],
    instructions: &str,
) -> Result<()> {
    for _ in 0..MAX_TOOL_CALLS {
        let response = api::call_openai(client, history, tools_defs, instructions).await?;
        let has_tool_calls = stream_response(response, history).await?;
        if !has_tool_calls {
            break;
        }
    }
    Ok(())
}

async fn stream_response(
    response: reqwest::Response,
    history: &mut Vec<serde_json::Value>,
) -> Result<bool> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut event_handler = EventHandler::new(history);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(data) = get_event(&mut buffer) {
            let Some(event) = parse_event(&data) else {
                continue;
            };

            event_handler.handle_event(event)?;
        }
    }

    Ok(event_handler.has_tool_calls())
}
