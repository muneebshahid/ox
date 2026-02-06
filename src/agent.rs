use crate::{api, tools};
use anyhow::Result;
use futures::StreamExt;
use serde::Deserialize;
use std::io::{self, Write};

const MAX_TOOL_CALLS: usize = 20;

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

fn parse_event(data: &str) -> Option<StreamEvent> {
    match serde_json::from_str(data) {
        Ok(event) => Some(event),
        Err(err) => {
            let preview: String = data.chars().take(200).collect();
            eprintln!("Warning: failed to parse SSE event JSON: {err}; payload: {preview}");
            None
        }
    }
}

fn get_event(buffer: &mut String) -> Option<String> {
    loop {
        let pos = buffer.find("\n\n")?;
        let event_text = buffer[..pos].to_string();
        *buffer = buffer[pos + 2..].to_string();

        let data_lines: Vec<String> = event_text
            .lines()
            .filter_map(|line| {
                let data = line.strip_prefix("data:")?;
                Some(data.strip_prefix(' ').unwrap_or(data).to_string())
            })
            .collect();

        if !data_lines.is_empty() {
            return Some(data_lines.join("\n"));
        }
    }
}

struct EventHandler<'a> {
    history: &'a mut Vec<serde_json::Value>,
    has_tool_calls: bool,
}

impl<'a> EventHandler<'a> {
    fn new(history: &'a mut Vec<serde_json::Value>) -> Self {
        Self {
            history,
            has_tool_calls: false,
        }
    }

    fn handle_event(&mut self, event: StreamEvent) -> Result<()> {
        match event {
            StreamEvent::OutputItemAdded { item } => self.handle_output_item_added(&item),
            StreamEvent::TextDelta { delta } => self.handle_text_delta(&delta)?,
            StreamEvent::OutputItemDone { item } => self.handle_output_item_done(&item),
            StreamEvent::Ignored => {}
        }

        Ok(())
    }

    fn handle_output_item_added(&self, item: &serde_json::Value) {
        if item["type"] == "function_call" {
            println!("Calling {}...", item["name"]);
        }
    }

    fn handle_text_delta(&self, delta: &str) -> Result<()> {
        print!("{delta}");
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_output_item_done(&mut self, item: &serde_json::Value) {
        match item["type"].as_str() {
            Some("message") => self.handle_output_message(item),
            Some("function_call") => self.handle_output_function_call(item),
            _ => {}
        }
    }

    fn handle_output_message(&mut self, item: &serde_json::Value) {
        println!();
        let text = item["content"][0]["text"].as_str().unwrap_or("");
        self.history.push(serde_json::json!({
            "role": "assistant",
            "content": text
        }));
    }

    fn handle_output_function_call(&mut self, item: &serde_json::Value) {
        let call_id = item["call_id"].as_str().unwrap_or("");
        let name = item["name"].as_str().unwrap_or("");
        let arguments = item["arguments"].as_str().unwrap_or("");
        let result = tools::execute(name, arguments);
        self.history.push(serde_json::json!({
            "type": "function_call",
            "call_id": call_id,
            "name": name,
            "arguments": arguments
        }));
        self.history.push(serde_json::json!({
            "type": "function_call_output",
            "call_id": call_id,
            "output": result
        }));
        self.has_tool_calls = true;
    }

    fn has_tool_calls(&self) -> bool {
        self.has_tool_calls
    }
}

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

#[cfg(test)]
mod tests {
    use super::{StreamEvent, get_event, parse_event};

    #[test]
    fn get_event_returns_first_data_payload() {
        let mut buffer = "event: response\ndata: {\"type\":\"x\"}\n\n".to_string();

        let data = get_event(&mut buffer);

        assert_eq!(data, Some("{\"type\":\"x\"}".to_string()));
        assert!(buffer.is_empty());
    }

    #[test]
    fn get_event_skips_non_data_events() {
        let mut buffer = "event: ping\n\n\
                          event: response\ndata: {\"type\":\"y\"}\n\n"
            .to_string();

        let data = get_event(&mut buffer);

        assert_eq!(data, Some("{\"type\":\"y\"}".to_string()));
        assert!(buffer.is_empty());
    }

    #[test]
    fn get_event_joins_multi_line_data() {
        let mut buffer = "event: response\n\
                          data: {\"type\":\"response.output_text.delta\",\n\
                          data: \"delta\":\"Hello\"}\n\n"
            .to_string();

        let data = get_event(&mut buffer);

        assert_eq!(
            data,
            Some("{\"type\":\"response.output_text.delta\",\n\"delta\":\"Hello\"}".to_string())
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_event_rejects_invalid_json() {
        let result = parse_event("{not-json");
        assert!(result.is_none());
    }

    #[test]
    fn parse_event_accepts_known_event() {
        let data = r#"{"type":"response.output_text.delta","delta":"hi"}"#;
        let event = parse_event(data).expect("event should parse");
        assert!(matches!(event, StreamEvent::TextDelta { .. }));
    }
}
