use std::borrow::Cow;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use crate::events::types::CoreEvent;

const STATUS_IDLE: &str = "Idle";
const STATUS_RUNNING: &str = "Running";

#[derive(Debug, PartialEq, Eq)]
pub enum UserInput {
    Insert(char),
    Backspace,
    Submit,
    Paste(String),
    Quit,
    Ignore,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StateCommand {
    None,
    Submit(String),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunPhase {
    Thinking,
    Responding,
    Tool { name: String },
}

pub struct TuiState {
    transcript: String,
    status: String,
    input: String,
    dirty: bool,
    running_started_at: Option<Instant>,
    run_phase: RunPhase,
    reasoning_trace_open: bool,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            transcript: String::new(),
            status: STATUS_IDLE.to_string(),
            input: String::new(),
            dirty: true,
            running_started_at: None,
            run_phase: RunPhase::Thinking,
            reasoning_trace_open: false,
        }
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn status_is_running(&self) -> bool {
        self.status == STATUS_RUNNING
    }

    pub fn running_phase_label(&self) -> Cow<'_, str> {
        match &self.run_phase {
            RunPhase::Thinking => Cow::Borrowed("Thinking"),
            RunPhase::Responding => Cow::Borrowed("Responding"),
            RunPhase::Tool { name } if name.is_empty() => Cow::Borrowed("Running tool"),
            RunPhase::Tool { name } => {
                Cow::Owned(format!("Running {}", truncate_preview(name, 24)))
            }
        }
    }

    pub fn running_elapsed(&self) -> Duration {
        self.running_started_at
            .as_ref()
            .map_or(Duration::ZERO, Instant::elapsed)
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn handle_agent_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::ShutdownRequested | CoreEvent::Tick => return,
            CoreEvent::AgentTurnStart => {
                self.set_running_status();
            }
            CoreEvent::AgentReasoningDelta(delta) => {
                self.run_phase = RunPhase::Thinking;
                self.append_reasoning_delta(&delta);
            }
            CoreEvent::AgentTextDelta(delta) => {
                self.close_reasoning_trace();
                if !matches!(self.run_phase, RunPhase::Responding) {
                    self.ensure_message_gap();
                }
                self.run_phase = RunPhase::Responding;
                self.transcript.push_str(&delta);
            }
            CoreEvent::AgentTurnEnd => {
                self.close_reasoning_trace();
                self.stop_running(STATUS_IDLE.to_string());
                if !self.transcript.ends_with('\n') {
                    self.transcript.push('\n');
                }
            }
            CoreEvent::AgentToolCallStart {
                tool_name, args, ..
            } => {
                self.close_reasoning_trace();
                self.run_phase = RunPhase::Tool {
                    name: tool_name.clone(),
                };
                let message = format_tool_start_message(&tool_name, &args);
                self.push_transcript_line(&message);
            }
            CoreEvent::AgentToolCallEnd { .. } => {
                self.run_phase = RunPhase::Thinking;
            }
            CoreEvent::Error(message) => {
                self.close_reasoning_trace();
                self.stop_running(message);
            }
        }
        self.mark_dirty();
    }

    pub fn handle_user_input(&mut self, input: UserInput) -> StateCommand {
        match input {
            UserInput::Insert(c) => {
                self.input.push(c);
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Backspace => {
                self.input.pop();
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Paste(pasted) => {
                self.input.push_str(&pasted);
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Submit => self.submit_input(),
            UserInput::Quit => StateCommand::Quit,
            UserInput::Ignore => StateCommand::None,
        }
    }

    fn submit_input(&mut self) -> StateCommand {
        let submitted = self.input.trim().to_string();
        self.input.clear();
        self.mark_dirty();

        if submitted.is_empty() {
            return StateCommand::None;
        }
        if submitted == "exit" {
            return StateCommand::Quit;
        }

        self.push_user_input(&submitted);
        self.set_running_status();
        self.mark_dirty();
        StateCommand::Submit(submitted)
    }

    fn set_running_status(&mut self) {
        if self.running_started_at.is_none() {
            self.running_started_at = Some(Instant::now());
        }
        self.status = STATUS_RUNNING.to_string();
        self.run_phase = RunPhase::Thinking;
    }

    fn stop_running(&mut self, status: String) {
        self.status = status;
        self.running_started_at = None;
        self.run_phase = RunPhase::Thinking;
    }

    fn append_reasoning_delta(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }

        if !self.reasoning_trace_open {
            self.ensure_message_gap();
            self.transcript.push_str("[thinking] ");
            self.reasoning_trace_open = true;
        }
        self.transcript.push_str(delta);
    }

    fn close_reasoning_trace(&mut self) {
        if self.reasoning_trace_open {
            if !self.transcript.ends_with('\n') {
                self.transcript.push('\n');
            }
            self.reasoning_trace_open = false;
        }
    }

    fn push_user_input(&mut self, input: &str) {
        self.ensure_message_gap();
        self.push_transcript_line(&format!("> {input}"));
    }

    fn push_transcript_line(&mut self, line: &str) {
        if !self.transcript.is_empty() && !self.transcript.ends_with('\n') {
            self.transcript.push('\n');
        }
        self.transcript.push_str(line);
        self.transcript.push('\n');
    }

    fn ensure_message_gap(&mut self) {
        if self.transcript.is_empty() || self.transcript.ends_with("\n\n") {
            return;
        }
        if self.transcript.ends_with('\n') {
            self.transcript.push('\n');
        } else {
            self.transcript.push_str("\n\n");
        }
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

fn format_tool_start_message(tool_name: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<serde_json::Value>(args_json).ok();

    if let Some(args) = args.as_ref()
        && let Some(message) = format_known_tool(tool_name, args)
    {
        return message;
    }

    let summary = args.as_ref().map_or_else(
        || truncate_preview(args_json, 120),
        |v| truncate_preview(&v.to_string(), 120),
    );
    if summary.is_empty() {
        format!("[tool] running {tool_name}")
    } else {
        format!("[tool] running {tool_name} {summary}")
    }
}

fn format_known_tool(tool_name: &str, args: &serde_json::Value) -> Option<String> {
    match tool_name {
        "read_file" => {
            let path = arg_str(args, "path")?;
            let mut message = format!("[tool] reading {}", truncate_preview(path, 100));
            match (arg_u64(args, "offset"), arg_u64(args, "limit")) {
                (Some(offset), Some(limit)) if limit > 0 => {
                    let end = offset.saturating_add(limit).saturating_sub(1);
                    let _ = write!(message, ":{offset}-{end}");
                }
                (Some(offset), None) => {
                    let _ = write!(message, ":{offset}-");
                }
                (None, Some(limit)) => {
                    let _ = write!(message, " (limit {limit})");
                }
                _ => {}
            }
            Some(message)
        }
        "ls" => {
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!("[tool] listing {}", truncate_preview(path, 100)))
        }
        "bash" => {
            let command = arg_str(args, "command")?.replace('\n', " ");
            Some(format!("[tool] bash: {}", truncate_preview(&command, 120)))
        }
        "write_file" => {
            let path = arg_str(args, "path")?;
            Some(format!("[tool] writing {}", truncate_preview(path, 100)))
        }
        "edit" => {
            let path = arg_str(args, "path")?;
            Some(format!("[tool] editing {}", truncate_preview(path, 100)))
        }
        "grep" => {
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!(
                "[tool] grep /{}/ in {}",
                truncate_preview(pattern, 60),
                truncate_preview(path, 80)
            ))
        }
        "find" => {
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!(
                "[tool] finding {} in {}",
                truncate_preview(pattern, 60),
                truncate_preview(path, 80)
            ))
        }
        _ => None,
    }
}

fn arg_str<'a>(args: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(serde_json::Value::as_str)
}

fn arg_u64(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(serde_json::Value::as_u64)
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
    use super::{StateCommand, TuiState, UserInput};
    use crate::events::types::CoreEvent;

    #[test]
    fn appends_deltas_and_updates_status() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTextDelta(" world".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);
        assert_eq!(state.status(), "Idle");
        assert_eq!(state.transcript(), "hello world\n");
        assert_eq!(state.input(), "");
        assert!(state.take_dirty());
    }

    #[test]
    fn submit_creates_command_and_echoes_transcript() {
        let mut state = TuiState::new();
        state.handle_user_input(UserInput::Insert('h'));
        state.handle_user_input(UserInput::Insert('i'));

        let command = state.handle_user_input(UserInput::Submit);
        assert_eq!(command, StateCommand::Submit("hi".to_string()));
        assert_eq!(state.transcript(), "> hi\n");
        assert_eq!(state.status(), "Running");
        assert_eq!(state.input(), "");
    }

    #[test]
    fn submit_exit_returns_quit() {
        let mut state = TuiState::new();
        state.handle_user_input(UserInput::Insert('e'));
        state.handle_user_input(UserInput::Insert('x'));
        state.handle_user_input(UserInput::Insert('i'));
        state.handle_user_input(UserInput::Insert('t'));

        let command = state.handle_user_input(UserInput::Submit);
        assert_eq!(command, StateCommand::Quit);
    }

    #[test]
    fn take_dirty_resets_flag() {
        let mut state = TuiState::new();
        assert!(state.take_dirty());
        assert!(!state.take_dirty());
    }

    #[test]
    fn shows_simple_tool_activity_for_read_file() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentToolCallStart {
            call_id: "call_1".to_string(),
            tool_name: "read_file".to_string(),
            args: r#"{"path":"src/main.rs","offset":10,"limit":5}"#.to_string(),
        });

        assert_eq!(state.transcript(), "[tool] reading src/main.rs:10-14\n");
    }

    #[test]
    fn running_phase_tracks_reasoning_text_and_tools() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        assert_eq!(state.running_phase_label(), "Thinking");

        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        assert_eq!(state.running_phase_label(), "Responding");

        state.handle_agent_event(CoreEvent::AgentReasoningDelta("step".to_string()));
        assert_eq!(state.running_phase_label(), "Thinking");

        state.handle_agent_event(CoreEvent::AgentToolCallStart {
            call_id: "call_1".to_string(),
            tool_name: "ls".to_string(),
            args: "{}".to_string(),
        });
        assert_eq!(state.running_phase_label(), "Running ls");
    }

    #[test]
    fn appends_reasoning_trace_from_reasoning_deltas() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentReasoningDelta("step one".to_string()));
        state.handle_agent_event(CoreEvent::AgentReasoningDelta(" + step two".to_string()));
        state.handle_agent_event(CoreEvent::AgentTextDelta("final".to_string()));

        assert_eq!(
            state.transcript(),
            "[thinking] step one + step two\n\nfinal"
        );
    }

    #[test]
    fn inserts_blank_line_between_user_and_assistant_messages() {
        let mut state = TuiState::new();
        state.handle_user_input(UserInput::Insert('h'));
        state.handle_user_input(UserInput::Insert('i'));
        let _ = state.handle_user_input(UserInput::Submit);

        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);

        assert_eq!(state.transcript(), "> hi\n\nhello\n");
    }

    #[test]
    fn inserts_blank_line_between_assistant_and_next_user_message() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);
        state.handle_user_input(UserInput::Paste("next".to_string()));
        let _ = state.handle_user_input(UserInput::Submit);

        assert_eq!(state.transcript(), "hello\n\n> next\n");
    }
}
