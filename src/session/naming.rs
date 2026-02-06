use anyhow::{Result, bail};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn validate_session_name(session_name: &str) -> Result<()> {
    if session_name.is_empty() {
        bail!("session name cannot be empty");
    }
    if !session_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        bail!("invalid session name '{session_name}' (allowed: a-z A-Z 0-9 - _ .)");
    }
    Ok(())
}

pub fn create_session_name() -> String {
    let epoch_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("session-{epoch_seconds}")
}
