use crate::{
    auth,
    events::{agent_bridge::AgentEventBridge, hub::EventHub},
    prompt, tools,
};

const CORE_EVENT_HUB_CAPACITY: usize = 1024;

pub struct AppContext {
    pub client: reqwest::Client,
    pub auth: auth::AuthConfig,
    pub tool_defs: Vec<serde_json::Value>,
    pub instructions: String,
    pub agent_bridge: AgentEventBridge,
}

impl AppContext {
    pub fn new() -> Self {
        let core_hub = EventHub::new(CORE_EVENT_HUB_CAPACITY);
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            auth: auth::AuthConfig::from_env(),
            tool_defs: tools::definitions(),
            instructions: prompt::build(),
            agent_bridge: AgentEventBridge::new(core_hub),
        }
    }
}
