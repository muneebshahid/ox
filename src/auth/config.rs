const OPENAI_API_URL: &str = "https://api.openai.com/v1/responses";
const SUBSCRIPTION_API_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Clone, Copy)]
pub(super) enum AuthMode {
    ApiKey,
    Subscription,
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

impl AuthConfig {
    pub fn from_env() -> Self {
        let mode = AuthMode::from_env_value(std::env::var("AUTH_MODE").ok().as_deref());
        let profile = auth_profile(mode);
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| profile.default_model.to_string());
        Self {
            mode,
            url: profile.url,
            model,
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
