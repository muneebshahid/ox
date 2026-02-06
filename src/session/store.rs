use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::naming::validate_session_name;

const SESSIONS_DIR: &str = ".sessions";
const SESSION_EXT: &str = "jsonl";

fn sessions_dir() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to determine current directory")?;
    Ok(cwd.join(SESSIONS_DIR))
}

pub(super) fn session_path(session_name: &str) -> Result<PathBuf> {
    validate_session_name(session_name)?;
    let dir = sessions_dir()?;
    Ok(dir.join(format!("{session_name}.{SESSION_EXT}")))
}

pub fn list_sessions() -> Result<Vec<String>> {
    let dir: PathBuf = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read sessions directory: {}", dir.display()))?
    {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(SESSION_EXT) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        sessions.push(stem.to_string());
    }

    sessions.sort();
    sessions.reverse();
    Ok(sessions)
}

pub(super) fn load_history_file(path: &Path) -> Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read session file: {}", path.display()))?;

    let mut history = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value) => history.push(value),
            Err(err) => eprintln!(
                "Warning: skipping malformed session line {} in {}: {}",
                line_no + 1,
                path.display(),
                err
            ),
        }
    }

    Ok(history)
}

pub(super) fn ensure_sessions_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create sessions directory: {}", parent.display())
        })?;
    }
    Ok(())
}
