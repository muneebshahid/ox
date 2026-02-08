const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";
const SUBSCRIPTION_API_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Clone, Copy)]
pub(super) enum AuthMode {
    ApiKey,
    Subscription,
}

#[derive(Clone, Copy)]
enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

#[derive(Clone, Copy)]
struct AuthProfile {
    url: &'static str,
    default_model: &'static str,
}

#[derive(Clone)]
pub struct AuthConfig {
    mode: AuthMode,
    url: &'static str,
    model: String,
    reasoning_effort: ReasoningEffort,
}

impl AuthMode {
    fn from_env_value(raw: Option<&str>) -> Self {
        match raw.unwrap_or("api").to_ascii_lowercase().as_str() {
            "subscription" => Self::Subscription,
            _ => Self::ApiKey,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "api",
            Self::Subscription => "subscription",
        }
    }
}

impl ReasoningEffort {
    fn from_env_value(raw: Option<&str>) -> Self {
        match raw.unwrap_or("").to_ascii_lowercase().as_str() {
            "minimal" => Self::Minimal,
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "xhigh" => Self::XHigh,
            // Reasoning is always on for ox; fallback is medium.
            "off" | "none" | "" => Self::Medium,
            _ => Self::Medium,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
        }
    }
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let mode = AuthMode::from_env_value(std::env::var("AUTH_MODE").ok().as_deref());
        let profile = auth_profile(mode);
        let model =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| profile.default_model.to_string());
        let reasoning_effort =
            ReasoningEffort::from_env_value(std::env::var("OPENAI_REASONING").ok().as_deref());
        Self {
            mode,
            url: profile.url,
            model,
            reasoning_effort,
        }
    }

    pub const fn mode_name(&self) -> &'static str {
        self.mode.as_str()
    }

    pub const fn model(&self) -> &str {
        self.model.as_str()
    }

    pub const fn endpoint(&self) -> &'static str {
        self.url
    }

    pub const fn reasoning_effort(&self) -> &'static str {
        self.reasoning_effort.as_str()
    }

    pub(super) const fn mode(&self) -> AuthMode {
        self.mode
    }
}

const fn auth_profile(mode: AuthMode) -> AuthProfile {
    match mode {
        AuthMode::ApiKey => AuthProfile {
            url: OPENAI_API_URL,
            default_model: "gpt-4.1-mini",
        },
        AuthMode::Subscription => AuthProfile {
            url: SUBSCRIPTION_API_URL,
            default_model: "gpt-5.3-codex",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::ReasoningEffort;

    #[test]
    fn parses_reasoning_effort_values() {
        assert_eq!(
            ReasoningEffort::from_env_value(Some("minimal")).as_str(),
            "minimal"
        );
        assert_eq!(ReasoningEffort::from_env_value(Some("low")).as_str(), "low");
        assert_eq!(
            ReasoningEffort::from_env_value(Some("medium")).as_str(),
            "medium"
        );
        assert_eq!(
            ReasoningEffort::from_env_value(Some("high")).as_str(),
            "high"
        );
        assert_eq!(
            ReasoningEffort::from_env_value(Some("xhigh")).as_str(),
            "xhigh"
        );
    }

    #[test]
    fn defaults_to_medium_for_empty_or_off() {
        assert_eq!(
            ReasoningEffort::from_env_value(Some("off")).as_str(),
            "medium"
        );
        assert_eq!(
            ReasoningEffort::from_env_value(Some("none")).as_str(),
            "medium"
        );
        assert_eq!(ReasoningEffort::from_env_value(Some("")).as_str(), "medium");
        assert_eq!(ReasoningEffort::from_env_value(None).as_str(), "medium");
    }

    #[test]
    fn defaults_to_medium_for_invalid_value() {
        assert_eq!(
            ReasoningEffort::from_env_value(Some("definitely-not-valid")).as_str(),
            "medium"
        );
    }
}
