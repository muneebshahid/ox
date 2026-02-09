use super::events::{OutputItem, OutputItemKind, StreamEvent};
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

    fn handle_output_item_added(item: &OutputItem) {
        if let OutputItemKind::FunctionCall { name, .. } = &item.parsed {
            println!("Calling {}...", name.as_deref().unwrap_or("unknown"));
        }
    }

    fn handle_text_delta(delta: &str) -> Result<()> {
        print!("{delta}");
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_output_item_done(&mut self, item: &OutputItem) {
        match &item.parsed {
            OutputItemKind::Message { .. } => self.handle_output_message(item),
            OutputItemKind::FunctionCall { .. } => self.handle_output_function_call(item),
            OutputItemKind::Reasoning => self.handle_output_reasoning(item),
            OutputItemKind::Other => {}
        }
    }

    fn handle_output_message(&mut self, item: &OutputItem) {
        println!();
        let text = match &item.parsed {
            OutputItemKind::Message { content } => content
                .first()
                .and_then(|part| part.text.as_deref())
                .unwrap_or(""),
            _ => "",
        };
        self.history.push(serde_json::json!({
            "role": "assistant",
            "content": text
        }));
    }

    fn handle_output_function_call(&mut self, item: &OutputItem) {
        let (call_id, name, arguments) = match &item.parsed {
            OutputItemKind::FunctionCall {
                call_id,
                name,
                arguments,
            } => (
                call_id.as_deref().unwrap_or(""),
                name.as_deref().unwrap_or(""),
                arguments.as_deref().unwrap_or(""),
            ),
            _ => ("", "", ""),
        };
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

    fn handle_output_reasoning(&mut self, item: &OutputItem) {
        self.history.push(item.raw.clone());
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
