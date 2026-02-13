use super::events::{
    FunctionCallItem, ResponseCompletedPayload, ResponseFailedPayload, StreamEvent,
    parse_function_call_item,
};
use crate::events::agent_bridge::AgentEventBridge;
use crate::tools;

pub(super) struct EventHandler<'a> {
    history: &'a mut Vec<serde_json::Value>,
    bridge: Option<&'a AgentEventBridge>,
    has_tool_calls: bool,
    committed_any: bool,
    saw_completed: bool,
    failure_message: Option<String>,
}

impl<'a> EventHandler<'a> {
    pub(super) const fn new(
        history: &'a mut Vec<serde_json::Value>,
        bridge: Option<&'a AgentEventBridge>,
    ) -> Self {
        Self {
            history,
            bridge,
            has_tool_calls: false,
            committed_any: false,
            saw_completed: false,
            failure_message: None,
        }
    }

    pub(super) fn handle_event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Ignored | StreamEvent::ReasoningSummaryPartAdded => {}
            StreamEvent::TextDelta { delta } => self.handle_text_delta(&delta),
            StreamEvent::ReasoningSummaryTextDelta { delta } => self.handle_reasoning_delta(&delta),
            StreamEvent::ReasoningSummaryPartDone => self.handle_reasoning_part_done(),
            StreamEvent::OutputItemDone { item } => self.handle_output_item_done(&item),
            StreamEvent::ResponseCompleted { response }
            | StreamEvent::ResponseDone { response } => {
                self.handle_response_completed(response.as_ref());
            }
            StreamEvent::ResponseFailed { response } => {
                self.handle_response_failed(response.as_ref());
            }
            StreamEvent::Error { code, message } => self.handle_error(code, message),
        }
    }

    pub(super) const fn has_tool_calls(&self) -> bool {
        self.has_tool_calls
    }

    pub(super) const fn committed_any(&self) -> bool {
        self.committed_any
    }

    pub(super) const fn saw_completed(&self) -> bool {
        self.saw_completed
    }

    pub(super) fn failure_message(&self) -> Option<&str> {
        self.failure_message.as_deref()
    }

    fn handle_text_delta(&self, delta: &str) {
        if let Some(bridge) = self.bridge {
            bridge.emit_text_delta(delta);
        }
    }

    fn handle_reasoning_delta(&self, delta: &str) {
        if let Some(bridge) = self.bridge {
            bridge.emit_reasoning_delta(delta);
        }
    }

    fn handle_reasoning_part_done(&self) {
        if let Some(bridge) = self.bridge {
            bridge.emit_reasoning_delta("\n\n");
        }
    }

    fn handle_output_item_done(&mut self, item: &serde_json::Value) {
        match item.get("type").and_then(serde_json::Value::as_str) {
            Some("message" | "reasoning") => {
                self.history.push(item.clone());
                self.committed_any = true;
            }
            Some("function_call") => self.handle_output_function_call(item),
            _ => {}
        }
    }

    fn handle_output_function_call(&mut self, item: &serde_json::Value) {
        let Some(FunctionCallItem {
            call_id,
            name,
            arguments,
        }) = parse_function_call_item(item)
        else {
            return;
        };

        let args = match arguments {
            serde_json::Value::String(s) => s,
            serde_json::Value::Null => String::new(),
            other => other.to_string(),
        };

        if let Some(bridge) = self.bridge {
            bridge.emit_tool_call_start(&call_id, &name, &args);
        }

        let result = tools::execute(&name, &args);

        if let Some(bridge) = self.bridge {
            bridge.emit_tool_call_end(&call_id, &name);
        }

        self.history.push(item.clone());
        self.history.push(serde_json::json!({
            "type": "function_call_output",
            "call_id": call_id,
            "output": result
        }));
        self.has_tool_calls = true;
        self.committed_any = true;
    }

    fn handle_response_completed(&mut self, response: Option<&ResponseCompletedPayload>) {
        self.saw_completed = true;
        if let Some(status) = response.and_then(|payload| payload.status.as_deref())
            && matches!(status, "failed" | "cancelled")
        {
            self.set_failure(format!("response.completed with status={status}"));
        }
    }

    #[allow(clippy::option_if_let_else)]
    fn handle_response_failed(&mut self, response: Option<&ResponseFailedPayload>) {
        let error = response.and_then(|p| p.error.as_ref());
        let message = if let Some(msg) = error.and_then(|e| e.message.as_deref()) {
            msg.to_owned()
        } else if let Some(code) = error.and_then(|e| e.code.as_deref()) {
            format!("response.failed: {code}")
        } else if let Some(status) = response.and_then(|p| p.status.as_deref()) {
            format!("response.failed with status={status}")
        } else {
            "response.failed event received".to_string()
        };
        self.set_failure(message);
    }

    fn handle_error(&mut self, code: Option<String>, message: Option<String>) {
        let detail = match (code, message) {
            (Some(code), Some(message)) => format!("{code}: {message}"),
            (Some(code), None) => code,
            (None, Some(message)) => message,
            (None, None) => "stream error event received".to_string(),
        };
        self.set_failure(detail);
    }

    fn set_failure(&mut self, message: String) {
        if self.failure_message.is_none() {
            self.failure_message = Some(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EventHandler;
    use crate::agent::events::StreamEvent;
    use crate::events::agent_bridge::AgentEventBridge;
    use crate::events::hub::EventHub;
    use crate::events::types::CoreEvent;

    #[test]
    fn stores_function_call_and_output_in_history() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.output_item.done",
                        "item": {
                            "type": "function_call",
                            "id": "fc_test",
                            "call_id": "call_test",
                            "name": "unknown_tool",
                            "arguments": "{}"
                        }
                    }"#,
            )
            .expect("parse function call event"),
        );

        assert!(handler.has_tool_calls());
        assert!(handler.committed_any());
        assert_eq!(history.len(), 2);
        assert_eq!(
            history[0],
            serde_json::json!({
                "type": "function_call",
                "id": "fc_test",
                "call_id": "call_test",
                "name": "unknown_tool",
                "arguments": "{}"
            })
        );
        assert_eq!(
            history[1],
            serde_json::json!({
                "type": "function_call_output",
                "call_id": "call_test",
                "output": "Unknown tool: unknown_tool"
            })
        );
    }

    #[test]
    fn stores_reasoning_item_in_history() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
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
        );

        assert!(handler.committed_any());
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

    #[test]
    fn marks_completed_on_response_completed_event() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.completed",
                        "response": { "status": "completed" }
                    }"#,
            )
            .expect("parse response.completed event"),
        );

        assert!(handler.saw_completed());
        assert!(handler.failure_message().is_none());
    }

    #[test]
    fn marks_failure_on_response_failed_event() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.failed",
                        "response": {
                            "status": "failed",
                            "error": { "code": "rate_limit_exceeded", "message": "try again later" }
                        }
                    }"#,
            )
            .expect("parse response.failed event"),
        );

        assert_eq!(handler.failure_message(), Some("try again later"));
    }

    #[test]
    fn marks_failure_on_error_event() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "error",
                        "code": "server_error",
                        "message": "internal error"
                    }"#,
            )
            .expect("parse error event"),
        );

        assert_eq!(
            handler.failure_message(),
            Some("server_error: internal error")
        );
    }

    #[test]
    fn does_not_mark_committed_on_delta_only_events() {
        let mut history = Vec::new();
        let mut handler = EventHandler::new(&mut history, None);

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.output_text.delta",
                        "delta": "hello"
                    }"#,
            )
            .expect("parse response.output_text.delta event"),
        );

        assert!(!handler.committed_any());
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn emits_tool_start_and_end_events() {
        let mut history = Vec::new();
        let hub = EventHub::new(8);
        let mut sub = hub.subscribe();
        let bridge = AgentEventBridge::new(hub);
        let mut handler = EventHandler::new(&mut history, Some(&bridge));

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.output_item.done",
                        "item": {
                            "type": "function_call",
                            "id": "fc_test",
                            "call_id": "call_test",
                            "name": "ls",
                            "arguments": "{\"path\":\".\"}"
                        }
                    }"#,
            )
            .expect("parse function call event"),
        );

        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentToolCallStart {
                call_id: "call_test".to_string(),
                tool_name: "ls".to_string(),
                args: "{\"path\":\".\"}".to_string(),
            })
        );
        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentToolCallEnd {
                call_id: "call_test".to_string(),
                tool_name: "ls".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn emits_reasoning_delta_event() {
        let mut history = Vec::new();
        let hub = EventHub::new(8);
        let mut sub = hub.subscribe();
        let bridge = AgentEventBridge::new(hub);
        let mut handler = EventHandler::new(&mut history, Some(&bridge));

        handler.handle_event(
            serde_json::from_str::<StreamEvent>(
                r#"{
                        "type": "response.reasoning_summary_text.delta",
                        "delta": "Analyzing..."
                    }"#,
            )
            .expect("parse reasoning summary delta event"),
        );

        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentReasoningDelta("Analyzing...".to_string()))
        );
    }
}
