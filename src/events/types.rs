#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreEvent {
    Tick,
    ShutdownRequested,
    AgentTurnStart,
    AgentTurnEnd,
    AgentTextDelta(String),
    AgentToolCallStart {
        call_id: String,
        tool_name: String,
        args: String,
    },
    AgentToolCallEnd {
        call_id: String,
        tool_name: String,
    },
    Error(String),
}
