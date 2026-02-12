use crate::events::types::CoreEvent;
use std::fmt::Write as _;

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

pub struct TuiState {
    transcript: String,
    status: String,
    input: String,
    dirty: bool,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            transcript: String::new(),
            status: "Idle".to_string(),
            input: String::new(),
            dirty: true,
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

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn handle_agent_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::AgentTurnStart => {
                self.status = "Running".to_string();
                self.mark_dirty();
            }
            CoreEvent::AgentTextDelta(delta) => {
                self.transcript.push_str(&delta);
                self.mark_dirty();
            }
            CoreEvent::AgentTurnEnd => {
                self.status = "Idle".to_string();
                if !self.transcript.ends_with('\n') {
                    self.transcript.push('\n');
                }
                self.mark_dirty();
            }
            CoreEvent::AgentToolCallStart {
                tool_name, args, ..
            } => {
                let message = format_tool_start_message(&tool_name, &args);
                self.push_transcript_line(&message);
                self.mark_dirty();
            }
            CoreEvent::AgentToolCallEnd { .. } | CoreEvent::Tick | CoreEvent::ShutdownRequested => {
            }
            CoreEvent::Error(message) => {
                self.status = message;
                self.mark_dirty();
            }
        }
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
        self.status = "Running".to_string();
        self.mark_dirty();
        StateCommand::Submit(submitted)
    }

    fn push_user_input(&mut self, input: &str) {
        self.push_transcript_line(&format!("> {input}"));
    }

    fn push_transcript_line(&mut self, line: &str) {
        if !self.transcript.is_empty() && !self.transcript.ends_with('\n') {
            self.transcript.push('\n');
        }
        self.transcript.push_str(line);
        self.transcript.push('\n');
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

fn format_tool_start_message(tool_name: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<serde_json::Value>(args_json).ok();

    match tool_name {
        "read_file" => {
            if let Some(args) = args.as_ref()
                && let Some(path) = arg_str(args, "path")
            {
                let mut message = format!("[tool] reading {}", truncate_preview(path, 100));
                if let Some(offset) = arg_u64(args, "offset") {
                    if let Some(limit) = arg_u64(args, "limit") {
                        if limit > 0 {
                            let end = offset.saturating_add(limit).saturating_sub(1);
                            let _ = write!(message, ":{offset}-{end}");
                        }
                    } else {
                        let _ = write!(message, ":{offset}-");
                    }
                } else if let Some(limit) = arg_u64(args, "limit") {
                    let _ = write!(message, " (limit {limit})");
                }
                return message;
            }
        }
        "ls" => {
            if let Some(args) = args.as_ref() {
                let path = arg_str(args, "path").unwrap_or(".");
                return format!("[tool] listing {}", truncate_preview(path, 100));
            }
        }
        "bash" => {
            if let Some(args) = args.as_ref()
                && let Some(command) = arg_str(args, "command")
            {
                let command = command.replace('\n', " ");
                return format!("[tool] bash: {}", truncate_preview(&command, 120));
            }
        }
        "write_file" => {
            if let Some(args) = args.as_ref()
                && let Some(path) = arg_str(args, "path")
            {
                return format!("[tool] writing {}", truncate_preview(path, 100));
            }
        }
        "edit" => {
            if let Some(args) = args.as_ref()
                && let Some(path) = arg_str(args, "path")
            {
                return format!("[tool] editing {}", truncate_preview(path, 100));
            }
        }
        "grep" => {
            if let Some(args) = args.as_ref() {
                let pattern = arg_str(args, "pattern").unwrap_or("");
                let path = arg_str(args, "path").unwrap_or(".");
                return format!(
                    "[tool] grep /{}/ in {}",
                    truncate_preview(pattern, 60),
                    truncate_preview(path, 80)
                );
            }
        }
        "find" => {
            if let Some(args) = args.as_ref() {
                let pattern = arg_str(args, "pattern").unwrap_or("");
                let path = arg_str(args, "path").unwrap_or(".");
                return format!(
                    "[tool] finding {} in {}",
                    truncate_preview(pattern, 60),
                    truncate_preview(path, 80)
                );
            }
        }
        _ => {}
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
}
