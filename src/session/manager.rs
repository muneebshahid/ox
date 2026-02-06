use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;

use super::store;

pub struct SessionManager {
    session_name: String,
    path: PathBuf,
    history: Vec<Value>,
}

impl SessionManager {
    pub fn open(session_name: &str) -> Result<Self> {
        let path = store::session_path(session_name)?;
        let history = store::load_history_file(&path)?;
        store::ensure_sessions_parent(&path)?;

        Ok(Self {
            session_name: session_name.to_string(),
            path,
            history,
        })
    }

    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    pub const fn history_mut(&mut self) -> &mut Vec<Value> {
        &mut self.history
    }

    pub const fn history_len(&self) -> usize {
        self.history.len()
    }

    fn append_line(&self, line: &str) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "failed to open session file for append: {}",
                    self.path.display()
                )
            })?;
        file.write_all(line.as_bytes()).with_context(|| {
            format!("failed to append session entry to {}", self.path.display())
        })?;
        file.write_all(b"\n")
            .with_context(|| format!("failed to append newline to {}", self.path.display()))?;
        Ok(())
    }

    pub fn append(&mut self, entry: Value) -> Result<()> {
        let line = serde_json::to_string(&entry).with_context(|| {
            format!(
                "failed to serialize session entry for {}",
                self.path.display()
            )
        })?;
        self.history.push(entry);
        self.append_line(&line)?;
        Ok(())
    }

    pub fn persist_from(&self, start: usize) -> Result<()> {
        if start >= self.history.len() {
            return Ok(());
        }

        for entry in &self.history[start..] {
            let line = serde_json::to_string(entry).with_context(|| {
                format!(
                    "failed to serialize session entry while persisting {}",
                    self.path.display()
                )
            })?;
            self.append_line(&line)?;
        }
        Ok(())
    }
}
