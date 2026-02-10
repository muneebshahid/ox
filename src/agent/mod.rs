mod event_handler;
mod events;
mod stream;
#[cfg(test)]
mod stream_tests;

use crate::api;
use crate::app_context::AppContext;
use anyhow::Result;
use event_handler::EventHandler;
use futures::StreamExt;
use stream::{get_event, parse_event};

const MAX_TOOL_CALLS: usize = 20;
const MAX_EMPTY_STREAM_RETRIES: u32 = 2;
const EMPTY_STREAM_BASE_DELAY_MS: u64 = 500;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct StreamError(String);

pub async fn run(
    app: &AppContext,
    history: &mut Vec<serde_json::Value>,
    session_id: &str,
) -> Result<()> {
    for _ in 0..MAX_TOOL_CALLS {
        let has_tool_calls = call_and_stream_with_retry(app, history, session_id).await?;
        if !has_tool_calls {
            break;
        }
    }
    Ok(())
}

async fn call_and_stream_with_retry(
    app: &AppContext,
    history: &mut Vec<serde_json::Value>,
    session_id: &str,
) -> Result<bool> {
    for attempt in 0..=MAX_EMPTY_STREAM_RETRIES {
        let response = api::call_openai(app, history, session_id).await?;
        match stream_response(response, history).await {
            Ok(result) => return Ok(result),
            Err(e)
                if attempt < MAX_EMPTY_STREAM_RETRIES
                    && e.downcast_ref::<StreamError>().is_some() =>
            {
                let delay = EMPTY_STREAM_BASE_DELAY_MS * 2u64.pow(attempt);
                eprintln!("stream failed, retrying in {delay}ms: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
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
            return Err(StreamError(format!("stream failed: {message}")).into());
        }
        if !handler.saw_completed() {
            return Err(StreamError("stream closed before response.completed".into()).into());
        }

        handler.has_tool_calls()
    };

    history.extend(staged_history);
    Ok(has_tool_calls)
}
