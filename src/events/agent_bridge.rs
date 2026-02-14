use crate::events::hub::{EventHub, Subscription};
use crate::events::types::CoreEvent;

#[derive(Clone)]
pub struct AgentEventBridge {
    hub: EventHub,
}

impl AgentEventBridge {
    pub const fn new(hub: EventHub) -> Self {
        Self { hub }
    }

    pub fn emit_turn_start(&self) {
        self.hub.publish(CoreEvent::AgentTurnStart);
    }

    pub fn emit_text_delta(&self, delta: &str) {
        self.hub
            .publish(CoreEvent::AgentTextDelta(delta.to_string()));
    }

    pub fn emit_reasoning_delta(&self, delta: &str) {
        self.hub
            .publish(CoreEvent::AgentReasoningDelta(delta.to_string()));
    }

    pub fn emit_tool_call_start(&self, call_id: &str, tool_name: &str, args: &str) {
        self.hub.publish(CoreEvent::AgentToolCallStart {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            args: args.to_string(),
        });
    }

    pub fn emit_tool_call_end(&self, call_id: &str, tool_name: &str) {
        self.hub.publish(CoreEvent::AgentToolCallEnd {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
        });
    }

    pub fn emit_error<S: Into<String>>(&self, message: S) {
        self.hub.publish(CoreEvent::Error(message.into()));
    }

    pub fn emit_turn_end(&self) {
        self.hub.publish(CoreEvent::AgentTurnEnd);
    }

    pub fn subscribe(&self) -> Subscription {
        self.hub.subscribe()
    }
}
