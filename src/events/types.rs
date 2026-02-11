#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreEvent {
    Tick,
    ShutdownRequested,
    AgentTurnStart,
    AgentTurnEnd,
    AgentTextDelta(String),
    Error(String),
}
