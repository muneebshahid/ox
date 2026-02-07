use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CodexAuthFile {
    pub(crate) tokens: Option<CodexTokens>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CodexTokens {
    pub(crate) access_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) account_id: Option<String>,
    pub(crate) id_token: Option<String>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

fn codex_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn auth_path() -> Result<PathBuf> {
    Ok(codex_home()?.join("auth.json"))
}

pub(super) fn load_auth_file() -> Result<CodexAuthFile> {
    let path = auth_path()?;
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
}

pub(super) fn save_auth_file(auth_file: &CodexAuthFile) -> Result<()> {
    let path = auth_path()?;
    let data = serde_json::to_string_pretty(auth_file)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    std::fs::write(&path, format!("{data}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}
