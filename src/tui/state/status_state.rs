use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

const STATUS_IDLE: &str = "Idle";
const STATUS_RUNNING: &str = "Running";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RunPhase {
    Thinking,
    Responding,
    Tool { name: String },
}

pub(super) struct StatusState {
    pub(super) text: String, // Status row text (for example `Idle`, `Running`, or error text).
    pub(super) running_started_at: Option<Instant>, // Start time for current running status.
    pub(super) run_phase: RunPhase, // Current running phase label shown in status.
}

impl StatusState {
    pub(super) fn new() -> Self {
        Self {
            text: STATUS_IDLE.to_string(),
            running_started_at: None,
            run_phase: RunPhase::Thinking,
        }
    }

    pub(super) fn is_running(&self) -> bool {
        self.text == STATUS_RUNNING
    }

    pub(super) fn is_visible(&self) -> bool {
        self.text != STATUS_IDLE
    }

    pub(super) fn set_running(&mut self) {
        if self.running_started_at.is_none() {
            self.running_started_at = Some(Instant::now());
        }
        self.text = STATUS_RUNNING.to_string();
        self.run_phase = RunPhase::Thinking;
    }

    pub(super) fn stop_running(&mut self, status: String) {
        self.text = status;
        self.running_started_at = None;
        self.run_phase = RunPhase::Thinking;
    }

    pub(super) fn running_phase_label(&self) -> Cow<'_, str> {
        match &self.run_phase {
            RunPhase::Thinking => Cow::Borrowed("Thinking"),
            RunPhase::Responding => Cow::Borrowed("Responding"),
            RunPhase::Tool { name } if name.is_empty() => Cow::Borrowed("Running tool"),
            RunPhase::Tool { name } => {
                Cow::Owned(format!("Running {}", truncate_preview(name, 24)))
            }
        }
    }

    pub(super) fn running_elapsed(&self) -> Duration {
        self.running_started_at
            .as_ref()
            .map_or(Duration::ZERO, Instant::elapsed)
    }
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max_chars).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::{RunPhase, StatusState};

    #[test]
    fn running_phase_label_truncates_long_tool_names() {
        let mut status = StatusState::new();
        status.set_running();
        status.run_phase = RunPhase::Tool {
            name: "a_very_long_tool_name_that_should_be_truncated".to_string(),
        };

        let label = status.running_phase_label();
        assert!(label.starts_with("Running "));
        assert!(label.ends_with("..."));
    }
}
