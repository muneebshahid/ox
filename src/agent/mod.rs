mod event_handler;
mod events;
mod stream;
#[cfg(test)]
mod stream_tests;

use crate::api;
use crate::app_context::AppContext;
use crate::events::agent_bridge::AgentEventBridge;
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
    bridge: &AgentEventBridge,
) -> Result<()> {
    for _ in 0..MAX_TOOL_CALLS {
        bridge.emit_turn_start();
        let turn_result = call_and_stream_with_retry(app, history, session_id, bridge).await;
        if let Err(err) = &turn_result {
            bridge.emit_error(err.to_string());
        }
        bridge.emit_turn_end();

        let has_tool_calls = turn_result?;
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
    bridge: &AgentEventBridge,
) -> Result<bool> {
    for attempt in 0..=MAX_EMPTY_STREAM_RETRIES {
        let response = api::call_openai(app, history, session_id).await?;
        match stream_response(response, history, bridge).await {
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
    bridge: &AgentEventBridge,
) -> Result<bool> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    let has_tool_calls = {
        let mut handler = EventHandler::new(history, Some(bridge));

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
            if !handler.committed_any() {
                return Err(StreamError(format!("stream failed: {message}")).into());
            }
            eprintln!("stream failed after committed output, continuing: {message}");
        }
        if !handler.saw_completed() {
            if !handler.committed_any() {
                return Err(StreamError("stream closed before response.completed".into()).into());
            }
            eprintln!("stream closed before response.completed after committed output, continuing");
        }

        handler.has_tool_calls()
    };

    Ok(has_tool_calls)
}

#[cfg(test)]
mod tests {
    use super::StreamError;
    use super::stream_response;
    use crate::events::agent_bridge::AgentEventBridge;
    use crate::events::hub::EventHub;
    use crate::events::types::CoreEvent;
    use anyhow::Result;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    async fn response_from_sse_body(body: &str) -> Result<reqwest::Response> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}"
        );

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept connection");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            let _ = socket.shutdown().await;
        });

        let url = format!("http://{addr}");
        Ok(reqwest::get(url).await?)
    }

    #[tokio::test]
    async fn stream_without_completed_and_without_commits_returns_stream_error() {
        let body = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n";
        let response = response_from_sse_body(body).await.expect("build response");
        let mut history = Vec::new();
        let bridge = AgentEventBridge::new(EventHub::new(16));

        let err = stream_response(response, &mut history, &bridge)
            .await
            .expect_err("expected stream error");
        assert!(
            err.downcast_ref::<StreamError>()
                .is_some_and(|e| e.to_string() == "stream closed before response.completed")
        );
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn stream_without_completed_after_message_done_keeps_committed_history() {
        let body = "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n";
        let response = response_from_sse_body(body).await.expect("build response");
        let mut history = Vec::new();
        let bridge = AgentEventBridge::new(EventHub::new(16));

        let has_tool_calls = stream_response(response, &mut history, &bridge)
            .await
            .expect("should not error after committed output");
        assert!(!has_tool_calls);
        assert_eq!(
            history,
            vec![serde_json::json!({
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Hello" }]
            })]
        );
    }

    #[tokio::test]
    async fn stream_failed_after_commit_keeps_committed_history() {
        let body = concat!(
            "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n",
            "data: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\",\"error\":{\"code\":\"server_error\",\"message\":\"boom\"}}}\n\n"
        );
        let response = response_from_sse_body(body).await.expect("build response");
        let mut history = Vec::new();
        let bridge = AgentEventBridge::new(EventHub::new(16));

        let has_tool_calls = stream_response(response, &mut history, &bridge)
            .await
            .expect("should not error after committed output");
        assert!(!has_tool_calls);
        assert_eq!(
            history,
            vec![serde_json::json!({
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Hello" }]
            })]
        );
    }

    #[tokio::test]
    async fn stream_emits_text_delta_event() {
        let body = concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n"
        );
        let response = response_from_sse_body(body).await.expect("build response");
        let mut history = Vec::new();
        let hub = EventHub::new(16);
        let mut sub = hub.subscribe();
        let bridge = AgentEventBridge::new(hub);

        let has_tool_calls = stream_response(response, &mut history, &bridge)
            .await
            .expect("stream should succeed");
        assert!(!has_tool_calls);

        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentTextDelta("Hi".to_string()))
        );
    }
}
