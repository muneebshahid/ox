use super::events::{OutputItem, StreamEvent};
use crate::tools;
use anyhow::Result;
use std::io::{self, Write};

pub(super) struct EventHandler<'a> {
    history: &'a mut Vec<serde_json::Value>,
    has_tool_calls: bool,
}

impl<'a> EventHandler<'a> {
    pub(super) fn new(history: &'a mut Vec<serde_json::Value>) -> Self {
        Self {
            history,
            has_tool_calls: false,
        }
    }

    pub(super) fn handle_event(&mut self, event: StreamEvent) -> Result<()> {
        match event {
            StreamEvent::OutputItemAdded { item } => self.handle_output_item_added(&item),
            StreamEvent::TextDelta { delta } => self.handle_text_delta(&delta)?,
            StreamEvent::OutputItemDone { item } => self.handle_output_item_done(&item),
            StreamEvent::Ignored => {}
        }

        Ok(())
    }

    pub(super) fn has_tool_calls(&self) -> bool {
        self.has_tool_calls
    }

    fn handle_output_item_added(&self, item: &OutputItem) {
        if item.item_type == "function_call" {
            println!("Calling {}...", item.name.as_deref().unwrap_or("unknown"));
        }
    }

    fn handle_text_delta(&self, delta: &str) -> Result<()> {
        print!("{delta}");
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_output_item_done(&mut self, item: &OutputItem) {
        match item.item_type.as_str() {
            "message" => self.handle_output_message(item),
            "function_call" => self.handle_output_function_call(item),
            _ => {}
        }
    }

    fn handle_output_message(&mut self, item: &OutputItem) {
        println!();
        let text = item
            .content
            .first()
            .and_then(|part| part.text.as_deref())
            .unwrap_or("");
        self.history.push(serde_json::json!({
            "role": "assistant",
            "content": text
        }));
    }

    fn handle_output_function_call(&mut self, item: &OutputItem) {
        let call_id = item.call_id.as_deref().unwrap_or("");
        let name = item.name.as_deref().unwrap_or("");
        let arguments = item.arguments.as_deref().unwrap_or("");
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
}
