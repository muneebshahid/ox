use std::borrow::Cow;
use std::time::{Duration, Instant};

use super::ui_action::UiAction;
use crate::events::types::CoreEvent;

mod tool_activity;

const STATUS_IDLE: &str = "Idle";
const STATUS_RUNNING: &str = "Running";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct InputBuffer {
    text: String,
    cursor: usize, // byte offset within `text`
}

impl InputBuffer {
    fn as_str(&self) -> &str {
        &self.text
    }

    const fn cursor(&self) -> usize {
        self.cursor
    }

    fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor = self.cursor.saturating_add(ch.len_utf8());
    }

    fn insert_str(&mut self, value: &str) {
        self.text.insert_str(self.cursor, value);
        self.cursor = self.cursor.saturating_add(value.len());
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let Some((start, _)) = self.text[..self.cursor].char_indices().last() else {
            return;
        };
        self.text.drain(start..self.cursor);
        self.cursor = start;
    }

    fn clear_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.text.drain(..self.cursor);
        self.cursor = 0;
    }

    fn clear_after_cursor(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        self.text.drain(self.cursor..);
    }

    fn delete_backward_word(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let mut start = self.cursor;
        start = rewind_while(&self.text, start, |ch| ch.is_whitespace());
        start = rewind_while(&self.text, start, |ch| !ch.is_whitespace());

        if start == self.cursor {
            return;
        }

        self.text.drain(start..self.cursor);
        self.cursor = start;
    }
}

fn rewind_while(text: &str, mut cursor: usize, predicate: impl Fn(char) -> bool) -> usize {
    while cursor > 0 {
        let prev_char_start = text[..cursor]
            .char_indices()
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let ch = text[prev_char_start..cursor].chars().next().unwrap_or('\0');
        if !predicate(ch) {
            break;
        }
        cursor = prev_char_start;
    }
    cursor
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
    input: InputBuffer,
    output_scroll_lines_from_bottom: u16,
    dirty: bool,
    running_started_at: Option<Instant>,
    run_phase: RunPhase,
    reasoning_trace_open: bool,
    interpret_backslash_enter_as_newline: bool,
    pending_backslash_enter_newline: bool,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            transcript: String::new(),
            status: STATUS_IDLE.to_string(),
            input: InputBuffer::default(),
            output_scroll_lines_from_bottom: 0,
            dirty: true,
            running_started_at: None,
            run_phase: RunPhase::Thinking,
            reasoning_trace_open: false,
            interpret_backslash_enter_as_newline: is_vscode_terminal(),
            pending_backslash_enter_newline: false,
        }
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn input(&self) -> &str {
        self.input.as_str()
    }

    pub(in crate::tui) const fn input_cursor(&self) -> usize {
        self.input.cursor()
    }

    pub const fn output_scroll_lines_from_bottom(&self) -> u16 {
        self.output_scroll_lines_from_bottom
    }

    pub(super) fn clamp_output_scroll_lines_from_bottom(&mut self, max_scroll_lines: u16) {
        self.output_scroll_lines_from_bottom =
            self.output_scroll_lines_from_bottom.min(max_scroll_lines);
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
                let message = tool_activity::format_tool_start(&tool_name, &args);
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

    pub fn handle_ui_action(&mut self, action: UiAction) -> StateCommand {
        match action {
            UiAction::Insert(c) => {
                self.input.insert_char(c);
                self.pending_backslash_enter_newline =
                    self.interpret_backslash_enter_as_newline && c == '\\';
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::Backspace => {
                self.input.backspace();
                self.pending_backslash_enter_newline = false;
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::DeleteBackwardWord => {
                self.input.delete_backward_word();
                self.pending_backslash_enter_newline = false;
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::ClearBeforeCursor => {
                self.input.clear_before_cursor();
                self.pending_backslash_enter_newline = false;
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::ClearAfterCursor => {
                self.input.clear_after_cursor();
                self.pending_backslash_enter_newline = false;
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::Paste(pasted) => {
                self.input.insert_str(&pasted);
                self.pending_backslash_enter_newline = false;
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::ScrollUp { lines } => {
                self.scroll_up(lines);
                StateCommand::None
            }
            UiAction::ScrollDown { lines } => {
                self.scroll_down(lines);
                StateCommand::None
            }
            UiAction::ViewportChanged => {
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::Submit => {
                if self.try_apply_backslash_enter_newline() {
                    return StateCommand::None;
                }
                self.submit_input()
            }
            UiAction::Quit => StateCommand::Quit,
            UiAction::Ignore => StateCommand::None,
        }
    }

    fn try_apply_backslash_enter_newline(&mut self) -> bool {
        if !self.interpret_backslash_enter_as_newline || !self.pending_backslash_enter_newline {
            return false;
        }
        self.pending_backslash_enter_newline = false;

        if self.input.cursor() != self.input.as_str().len() {
            return false;
        }
        if !self.input.as_str().ends_with('\\') {
            return false;
        }

        self.input.backspace();
        self.input.insert_char('\n');
        self.mark_dirty();
        true
    }

    fn submit_input(&mut self) -> StateCommand {
        let submitted = self.input.as_str().trim().to_string();
        self.input.clear();
        self.pending_backslash_enter_newline = false;
        self.mark_dirty();

        if submitted.is_empty() {
            return StateCommand::None;
        }
        if submitted == "exit" {
            return StateCommand::Quit;
        }

        self.output_scroll_lines_from_bottom = 0;
        self.push_user_message(&submitted);
        self.set_running_status();
        self.mark_dirty();
        StateCommand::Submit(submitted)
    }

    const fn scroll_up(&mut self, lines: u16) {
        if lines == 0 {
            return;
        }
        self.output_scroll_lines_from_bottom =
            self.output_scroll_lines_from_bottom.saturating_add(lines);
        self.mark_dirty();
    }

    const fn scroll_down(&mut self, lines: u16) {
        if lines == 0 {
            return;
        }
        self.output_scroll_lines_from_bottom =
            self.output_scroll_lines_from_bottom.saturating_sub(lines);
        self.mark_dirty();
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

    fn push_user_message(&mut self, input: &str) {
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

fn truncate_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max_chars).collect();
    format!("{truncated}...")
}

fn is_vscode_terminal() -> bool {
    std::env::var("TERM_PROGRAM").is_ok_and(|value| value.eq_ignore_ascii_case("vscode"))
        || std::env::var("VSCODE_PID").is_ok()
}

#[cfg(test)]
mod tests {
    use super::{StateCommand, TuiState, truncate_preview};
    use crate::events::types::CoreEvent;
    use crate::tui::ui_action::UiAction;

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
        state.handle_ui_action(UiAction::Insert('h'));
        state.handle_ui_action(UiAction::Insert('i'));

        let command = state.handle_ui_action(UiAction::Submit);
        assert_eq!(command, StateCommand::Submit("hi".to_string()));
        assert_eq!(state.transcript(), "> hi\n");
        assert_eq!(state.status(), "Running");
        assert_eq!(state.input(), "");
    }

    #[test]
    fn ctrl_u_clears_before_cursor_and_ctrl_k_clears_after_cursor() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("hello world".to_string()));

        let _ = state.handle_ui_action(UiAction::ClearAfterCursor);
        assert_eq!(state.input(), "hello world");

        let _ = state.handle_ui_action(UiAction::ClearBeforeCursor);
        assert_eq!(state.input(), "");
    }

    #[test]
    fn alt_backspace_deletes_one_word() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("hello world".to_string()));
        state.handle_ui_action(UiAction::DeleteBackwardWord);
        assert_eq!(state.input(), "hello ");
        state.handle_ui_action(UiAction::DeleteBackwardWord);
        assert_eq!(state.input(), "");
    }

    #[test]
    fn backslash_then_enter_inserts_newline_in_vscode_terminals() {
        let mut state = TuiState::new();
        state.interpret_backslash_enter_as_newline = true;

        state.handle_ui_action(UiAction::Insert('\\'));
        let command = state.handle_ui_action(UiAction::Submit);

        assert_eq!(command, StateCommand::None);
        assert_eq!(state.input(), "\n");
    }

    #[test]
    fn submit_exit_returns_quit() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Insert('e'));
        state.handle_ui_action(UiAction::Insert('x'));
        state.handle_ui_action(UiAction::Insert('i'));
        state.handle_ui_action(UiAction::Insert('t'));

        let command = state.handle_ui_action(UiAction::Submit);
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
        state.handle_ui_action(UiAction::Insert('h'));
        state.handle_ui_action(UiAction::Insert('i'));
        let _ = state.handle_ui_action(UiAction::Submit);

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
        state.handle_ui_action(UiAction::Paste("next".to_string()));
        let _ = state.handle_ui_action(UiAction::Submit);

        assert_eq!(state.transcript(), "hello\n\n> next\n");
    }

    #[test]
    fn scroll_input_moves_output_offset_and_clamps_at_zero_on_down() {
        let mut state = TuiState::new();
        let _ = state.take_dirty();

        state.handle_ui_action(UiAction::ScrollUp { lines: 5 });
        assert_eq!(state.output_scroll_lines_from_bottom(), 5);
        assert!(state.take_dirty());

        state.handle_ui_action(UiAction::ScrollDown { lines: 2 });
        assert_eq!(state.output_scroll_lines_from_bottom(), 3);

        state.handle_ui_action(UiAction::ScrollDown { lines: 10 });
        assert_eq!(state.output_scroll_lines_from_bottom(), 0);
    }

    #[test]
    fn submit_resets_manual_scroll_back_to_follow_output() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::ScrollUp { lines: 4 });
        state.handle_ui_action(UiAction::Paste("hello".to_string()));

        let command = state.handle_ui_action(UiAction::Submit);

        assert_eq!(command, StateCommand::Submit("hello".to_string()));
        assert_eq!(state.output_scroll_lines_from_bottom(), 0);
    }

    #[test]
    fn viewport_change_marks_state_dirty() {
        let mut state = TuiState::new();
        let _ = state.take_dirty();

        state.handle_ui_action(UiAction::ViewportChanged);

        assert!(state.take_dirty());
    }

    #[test]
    fn clamps_scroll_offset_to_current_max_when_content_cannot_scroll_further() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::ScrollUp { lines: 100 });
        assert_eq!(state.output_scroll_lines_from_bottom(), 100);

        state.clamp_output_scroll_lines_from_bottom(5);

        assert_eq!(state.output_scroll_lines_from_bottom(), 5);
    }

    #[test]
    fn truncate_preview_appends_ellipsis_when_exceeding_limit() {
        assert_eq!(truncate_preview("hello", 5), "hello");
        assert_eq!(truncate_preview("hello world", 5), "hello...");
    }

    #[test]
    fn running_phase_label_truncates_long_tool_names() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentToolCallStart {
            call_id: "call_1".to_string(),
            tool_name: "a_very_long_tool_name_that_should_be_truncated".to_string(),
            args: "{}".to_string(),
        });

        let label = state.running_phase_label();
        assert!(label.starts_with("Running "));
        assert!(label.ends_with("..."));
    }
}
