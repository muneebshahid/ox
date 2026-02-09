use super::events::{StreamEvent, parse_function_call_item};
use crate::tools;
use anyhow::Result;
use std::io::{self, Write};

pub(super) struct EventHandler<'a> {
    history: &'a mut Vec<serde_json::Value>,
    has_tool_calls: bool,
}

impl<'a> EventHandler<'a> {
    pub(super) const fn new(history: &'a mut Vec<serde_json::Value>) -> Self {
        Self {
            history,
            has_tool_calls: false,
        }
    }

    pub(super) fn handle_event(&mut self, event: StreamEvent) -> Result<()> {
        match event {
            StreamEvent::OutputItemAdded { item } => Self::handle_output_item_added(&item),
            StreamEvent::TextDelta { delta } => Self::handle_text_delta(&delta)?,
            StreamEvent::OutputItemDone { item } => self.handle_output_item_done(&item),
            StreamEvent::Ignored => {}
        }

        Ok(())
    }

    pub(super) const fn has_tool_calls(&self) -> bool {
        self.has_tool_calls
    }

    fn handle_output_item_added(item: &serde_json::Value) {
        if let Some(call) = parse_function_call_item(item) {
            let name = if call.name.is_empty() {
                "unknown"
            } else {
                call.name.as_str()
            };
            println!("Calling {}...", name);
        }
    }

    fn handle_text_delta(delta: &str) -> Result<()> {
        print!("{delta}");
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_output_item_done(&mut self, item: &serde_json::Value) {
        match item.get("type").and_then(serde_json::Value::as_str) {
            Some("message") | Some("reasoning") => self.history.push(item.clone()),
            Some("function_call") => self.handle_output_function_call(item),
            _ => {}
        }
    }

    fn handle_output_function_call(&mut self, item: &serde_json::Value) {
        let Some(call) = parse_function_call_item(item) else {
            return;
        };

        let call_id = call.call_id;
        let name = call.name;
        let arguments = match call.arguments {
            serde_json::Value::String(value) => value,
            serde_json::Value::Null => String::new(),
            value => value.to_string(),
        };

        let result = tools::execute(&name, &arguments);
        self.history.push(item.clone());
        self.history.push(serde_json::json!({
            "type": "function_call_output",
            "call_id": call_id,
            "output": result
        }));
        self.has_tool_calls = true;
    }
}

#[cfg(test)]
mod tests {
    use super::EventHandler;
    use crate::agent::events::StreamEvent;

    #[test]
    fn stores_reasoning_item_in_history() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history);

        handler
            .handle_event(
                serde_json::from_str::<StreamEvent>(
                    r#"{
                        "type": "response.output_item.done",
                        "item": {
                            "type": "reasoning",
                            "summary": [
                                { "type": "summary_text", "text": "step one" },
                                { "type": "summary_text", "text": "step two" }
                            ],
                            "encrypted_content": "abc123",
                            "extra_field": "keep-me"
                        }
                    }"#,
                )
                .expect("parse reasoning event"),
            )
            .expect("handle reasoning event");

        assert_eq!(history.len(), 1);
        assert_eq!(
            history[0],
            serde_json::json!({
                "type": "reasoning",
                "summary": [
                    { "type": "summary_text", "text": "step one" },
                    { "type": "summary_text", "text": "step two" }
                ],
                "encrypted_content": "abc123",
                "extra_field": "keep-me"
            })
        );
    }
}
