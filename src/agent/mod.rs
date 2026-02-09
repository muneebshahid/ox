mod event_handler;
mod events;
mod stream;
#[cfg(test)]
mod stream_tests;

use crate::api;
use crate::app_context::AppContext;
use anyhow::{Result, anyhow};
use event_handler::EventHandler;
use futures::StreamExt;
use stream::{get_event, parse_event};

const MAX_TOOL_CALLS: usize = 20;

pub async fn run(
    app: &AppContext,
    history: &mut Vec<serde_json::Value>,
    session_id: &str,
) -> Result<()> {
    for _ in 0..MAX_TOOL_CALLS {
        let response = api::call_openai(app, history, session_id).await?;
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
    let mut staged_history = Vec::new();

    let has_tool_calls = {
        let mut handler = EventHandler::new(&mut staged_history);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(data) = get_event(&mut buffer) {
                let Some(event) = parse_event(&data) else {
                    continue;
                };
                handler.handle_event(event)?;
            }
        }

        if let Some(message) = handler.failure_message() {
            return Err(anyhow!("stream failed: {message}"));
        }
        if !handler.saw_completed() {
            return Err(anyhow!("stream closed before response.completed"));
        }

        handler.has_tool_calls()
    };

    history.extend(staged_history);
    Ok(has_tool_calls)
}
