use crate::{auth, prompt, tools};

pub struct AppContext {
    pub client: reqwest::Client,
    pub auth: auth::AuthConfig,
    pub tool_defs: Vec<serde_json::Value>,
    pub instructions: String,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            auth: auth::AuthConfig::from_env(),
            tool_defs: tools::definitions(),
            instructions: prompt::build(),
        }
    }
}
