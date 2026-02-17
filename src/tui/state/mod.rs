use std::borrow::Cow;
use std::time::{Duration, Instant};

use super::{
    output_surface::{CellPos, OutputViewport},
    ui_action::UiAction,
};
use crate::events::types::CoreEvent;
use input_buffer::InputBuffer;

mod input_buffer;
mod input_cursor;
mod tool_activity;

const STATUS_IDLE: &str = "Idle";
const STATUS_RUNNING: &str = "Running";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputSelection {
    anchor: CellPos,
    focus: CellPos,
    selecting: bool,
    pending_copy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct LayoutContext {
    input_inner_width: u16,
    max_output_scroll_lines_from_bottom: u16,
    output_viewport: OutputViewport,
}

impl LayoutContext {
    pub(in crate::tui) const fn new(
        input_inner_width: u16,
        max_output_scroll_lines_from_bottom: u16,
        output_viewport: OutputViewport,
    ) -> Self {
        Self {
            input_inner_width,
            max_output_scroll_lines_from_bottom,
            output_viewport,
        }
    }

    pub(in crate::tui) const fn empty() -> Self {
        Self {
            input_inner_width: 0,
            max_output_scroll_lines_from_bottom: 0,
            output_viewport: OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        }
    }
}

struct InputState {
    buffer: InputBuffer, // Editable input text and cursor byte-offset state.
}

impl InputState {
    fn new() -> Self {
        Self {
            buffer: InputBuffer::new(),
        }
    }
}

struct OutputState {
    log: String,                        // Append-only output log shown in the output pane.
    scroll_lines_from_bottom: u16,      // Manual scroll distance measured from bottom.
    reasoning_trace_open: bool,         // Whether `[thinking]` trace is currently open.
    selection: Option<OutputSelection>, // Active or completed output selection state.
}

impl OutputState {
    const fn new() -> Self {
        Self {
            log: String::new(),
            scroll_lines_from_bottom: 0,
            reasoning_trace_open: false,
            selection: None,
        }
    }
}

struct StatusState {
    text: String, // Status row text (for example `Idle`, `Running`, or error text).
    running_started_at: Option<Instant>, // Start time for current running status.
    run_phase: RunPhase, // Current running phase label shown in status.
}

impl StatusState {
    fn new() -> Self {
        Self {
            text: STATUS_IDLE.to_string(),
            running_started_at: None,
            run_phase: RunPhase::Thinking,
        }
    }

    fn is_running(&self) -> bool {
        self.text == STATUS_RUNNING
    }

    fn is_visible(&self) -> bool {
        self.text != STATUS_IDLE
    }
}

pub struct TuiState {
    input: InputState,     // Input-specific state and layout metadata.
    output: OutputState,   // Output log text and output-scroll state.
    status: StatusState,   // Running/phase/status metadata for the status row.
    layout: LayoutContext, // Latest render/layout context computed by UI runtime.
    dirty: bool,           // Marks whether a redraw is needed.
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            output: OutputState::new(),
            status: StatusState::new(),
            layout: LayoutContext::empty(),
            dirty: true,
        }
    }

    pub fn output_log(&self) -> &str {
        &self.output.log
    }

    pub fn status(&self) -> &str {
        &self.status.text
    }

    pub fn input(&self) -> &str {
        self.input.buffer.text()
    }

    pub(in crate::tui) fn input_cursor_text(&self) -> &str {
        self.input.buffer.cursor_text()
    }

    pub(in crate::tui) fn output_selection_range(&self) -> Option<(CellPos, CellPos)> {
        let selection = self.output.selection?;
        let first = selection.anchor;
        let second = selection.focus;
        if (second.row, second.col) < (first.row, first.col) {
            Some((second, first))
        } else {
            Some((first, second))
        }
    }

    pub(in crate::tui) fn take_pending_copy_range(&mut self) -> Option<(CellPos, CellPos)> {
        let selection = self.output.selection?;
        if !selection.pending_copy {
            return None;
        }
        if selection.anchor == selection.focus {
            self.output.selection = Some(OutputSelection {
                pending_copy: false,
                ..selection
            });
            return None;
        }

        let (start, end) = self.output_selection_range()?;
        self.output.selection = Some(OutputSelection {
            pending_copy: false,
            ..selection
        });
        Some((start, end))
    }

    pub const fn output_scroll_lines_from_bottom(&self) -> u16 {
        self.output.scroll_lines_from_bottom
    }

    pub(in crate::tui) fn set_layout_context(&mut self, layout: LayoutContext) {
        self.layout = layout;
        self.output.scroll_lines_from_bottom = self
            .output
            .scroll_lines_from_bottom
            .min(layout.max_output_scroll_lines_from_bottom);
    }

    pub fn status_is_running(&self) -> bool {
        self.status.is_running()
    }

    pub(in crate::tui) fn status_row_visible(&self) -> bool {
        self.status.is_visible()
    }

    pub fn running_phase_label(&self) -> Cow<'_, str> {
        match &self.status.run_phase {
            RunPhase::Thinking => Cow::Borrowed("Thinking"),
            RunPhase::Responding => Cow::Borrowed("Responding"),
            RunPhase::Tool { name } if name.is_empty() => Cow::Borrowed("Running tool"),
            RunPhase::Tool { name } => {
                Cow::Owned(format!("Running {}", truncate_preview(name, 24)))
            }
        }
    }

    pub fn running_elapsed(&self) -> Duration {
        self.status
            .running_started_at
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
                self.status.run_phase = RunPhase::Thinking;
                self.append_reasoning_delta(&delta);
            }
            CoreEvent::AgentTextDelta(delta) => {
                self.close_reasoning_trace();
                if !matches!(self.status.run_phase, RunPhase::Responding) {
                    self.ensure_message_gap();
                }
                self.status.run_phase = RunPhase::Responding;
                self.output.log.push_str(&delta);
            }
            CoreEvent::AgentTurnEnd => {
                self.close_reasoning_trace();
                self.stop_running(STATUS_IDLE.to_string());
                if !self.output.log.ends_with('\n') {
                    self.output.log.push('\n');
                }
            }
            CoreEvent::AgentToolCallStart {
                tool_name, args, ..
            } => {
                self.close_reasoning_trace();
                self.status.run_phase = RunPhase::Tool {
                    name: tool_name.clone(),
                };
                let message = tool_activity::format_tool_start(&tool_name, &args);
                self.push_output_log_line(&message);
            }
            CoreEvent::AgentToolCallEnd { .. } => {
                self.status.run_phase = RunPhase::Thinking;
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
                self.input.buffer.insert_char(c);
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::Backspace => self.finish_input_mutation(InputBuffer::backspace),
            UiAction::Delete => self.finish_input_mutation(InputBuffer::delete_forward),
            UiAction::DeleteToLineStart => {
                self.finish_input_mutation(InputBuffer::delete_to_line_start)
            }
            UiAction::DeleteToLineEnd => {
                self.finish_input_mutation(InputBuffer::delete_to_line_end)
            }
            UiAction::MoveCursorLeft => self.finish_input_mutation(InputBuffer::move_left),
            UiAction::MoveCursorRight => self.finish_input_mutation(InputBuffer::move_right),
            UiAction::MoveCursorUp => {
                let width = self.layout.input_inner_width;
                self.finish_input_mutation(|input| input.move_up(width))
            }
            UiAction::MoveCursorDown => {
                let width = self.layout.input_inner_width;
                self.finish_input_mutation(|input| input.move_down(width))
            }
            UiAction::MoveCursorWordLeft => self.finish_input_mutation(InputBuffer::move_word_left),
            UiAction::MoveCursorWordRight => {
                self.finish_input_mutation(InputBuffer::move_word_right)
            }
            UiAction::MoveCursorLineStart => {
                self.finish_input_mutation(InputBuffer::move_line_start)
            }
            UiAction::MoveCursorLineEnd => self.finish_input_mutation(InputBuffer::move_line_end),
            UiAction::DeleteWordLeft => self.finish_input_mutation(InputBuffer::delete_word_left),
            UiAction::Paste(pasted) => {
                self.input.buffer.paste(&pasted);
                self.finish_input_edit(!pasted.is_empty())
            }
            UiAction::ScrollUp { lines } => {
                self.scroll_up(lines, self.layout.max_output_scroll_lines_from_bottom);
                StateCommand::None
            }
            UiAction::ScrollDown { lines } => {
                self.scroll_down(lines);
                StateCommand::None
            }
            UiAction::OutputSelectStart { col, row } => {
                self.begin_output_selection(col, row, self.layout.output_viewport);
                StateCommand::None
            }
            UiAction::OutputSelectDrag { col, row } => {
                self.update_output_selection(col, row, self.layout.output_viewport);
                StateCommand::None
            }
            UiAction::OutputSelectEnd { col, row } => {
                self.end_output_selection(col, row, self.layout.output_viewport);
                StateCommand::None
            }
            UiAction::ViewportChanged => {
                self.mark_dirty();
                StateCommand::None
            }
            UiAction::Submit => self.submit_input(),
            UiAction::Quit => StateCommand::Quit,
            UiAction::Ignore => StateCommand::None,
        }
    }

    const fn finish_input_edit(&mut self, changed: bool) -> StateCommand {
        if changed {
            self.mark_dirty();
        }
        StateCommand::None
    }

    fn finish_input_mutation(
        &mut self,
        mutator: impl FnOnce(&mut InputBuffer) -> bool,
    ) -> StateCommand {
        let changed = mutator(&mut self.input.buffer);
        self.finish_input_edit(changed)
    }

    fn submit_input(&mut self) -> StateCommand {
        let submitted = self.input.buffer.text().trim().to_string();
        self.input.buffer.clear();
        self.mark_dirty();

        if submitted.is_empty() {
            return StateCommand::None;
        }
        if submitted == "exit" {
            return StateCommand::Quit;
        }

        self.output.scroll_lines_from_bottom = 0;
        self.push_user_message(&submitted);
        self.set_running_status();
        self.mark_dirty();
        StateCommand::Submit(submitted)
    }

    fn scroll_up(&mut self, lines: u16, max_scroll_lines_from_bottom: u16) {
        if lines == 0 {
            return;
        }
        self.output.scroll_lines_from_bottom = self
            .output
            .scroll_lines_from_bottom
            .saturating_add(lines)
            .min(max_scroll_lines_from_bottom);
        self.mark_dirty();
    }

    const fn scroll_down(&mut self, lines: u16) {
        if lines == 0 {
            return;
        }
        self.output.scroll_lines_from_bottom =
            self.output.scroll_lines_from_bottom.saturating_sub(lines);
        self.mark_dirty();
    }

    const fn begin_output_selection(&mut self, col: u16, row: u16, viewport: OutputViewport) {
        let Some(relative) = relative_cell_in_viewport(viewport, col, row) else {
            if self.output.selection.is_some() {
                self.output.selection = None;
                self.mark_dirty();
            }
            return;
        };

        self.output.selection = Some(OutputSelection {
            anchor: relative,
            focus: relative,
            selecting: true,
            pending_copy: false,
        });
        self.mark_dirty();
    }

    const fn update_output_selection(&mut self, col: u16, row: u16, viewport: OutputViewport) {
        let Some(selection) = self.output.selection else {
            return;
        };
        if !selection.selecting {
            return;
        }

        let focus = clamp_to_viewport(viewport, col, row);
        self.output.selection = Some(OutputSelection { focus, ..selection });
        self.mark_dirty();
    }

    fn end_output_selection(&mut self, col: u16, row: u16, viewport: OutputViewport) {
        self.update_output_selection(col, row, viewport);
        let Some(selection) = self.output.selection else {
            return;
        };
        if !selection.selecting {
            return;
        }
        self.output.selection = Some(OutputSelection {
            selecting: false,
            pending_copy: selection.anchor != selection.focus,
            ..selection
        });
        self.mark_dirty();
    }

    fn set_running_status(&mut self) {
        if self.status.running_started_at.is_none() {
            self.status.running_started_at = Some(Instant::now());
        }
        self.status.text = STATUS_RUNNING.to_string();
        self.status.run_phase = RunPhase::Thinking;
    }

    fn stop_running(&mut self, status: String) {
        self.status.text = status;
        self.status.running_started_at = None;
        self.status.run_phase = RunPhase::Thinking;
    }

    fn append_reasoning_delta(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }

        if !self.output.reasoning_trace_open {
            self.ensure_message_gap();
            self.output.log.push_str("[thinking] ");
            self.output.reasoning_trace_open = true;
        }
        self.output.log.push_str(delta);
    }

    fn close_reasoning_trace(&mut self) {
        if self.output.reasoning_trace_open {
            if !self.output.log.ends_with('\n') {
                self.output.log.push('\n');
            }
            self.output.reasoning_trace_open = false;
        }
    }

    fn push_user_message(&mut self, input: &str) {
        self.ensure_message_gap();
        self.push_output_log_line(&format!("> {input}"));
    }

    fn push_output_log_line(&mut self, line: &str) {
        if !self.output.log.is_empty() && !self.output.log.ends_with('\n') {
            self.output.log.push('\n');
        }
        self.output.log.push_str(line);
        self.output.log.push('\n');
    }

    fn ensure_message_gap(&mut self) {
        if self.output.log.is_empty() || self.output.log.ends_with("\n\n") {
            return;
        }
        if self.output.log.ends_with('\n') {
            self.output.log.push('\n');
        } else {
            self.output.log.push_str("\n\n");
        }
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

const fn relative_cell_in_viewport(
    viewport: OutputViewport,
    col: u16,
    row: u16,
) -> Option<CellPos> {
    if viewport.width == 0 || viewport.height == 0 {
        return None;
    }
    if col < viewport.x
        || row < viewport.y
        || col >= viewport.x.saturating_add(viewport.width)
        || row >= viewport.y.saturating_add(viewport.height)
    {
        return None;
    }

    Some(CellPos {
        col: col - viewport.x,
        row: row - viewport.y,
    })
}

const fn clamp_to_viewport(viewport: OutputViewport, col: u16, row: u16) -> CellPos {
    let max_col = viewport.width.saturating_sub(1);
    let max_row = viewport.height.saturating_sub(1);

    let clamped_col = if col < viewport.x {
        0
    } else if col >= viewport.x.saturating_add(viewport.width) {
        max_col
    } else {
        col - viewport.x
    };
    let clamped_row = if row < viewport.y {
        0
    } else if row >= viewport.y.saturating_add(viewport.height) {
        max_row
    } else {
        row - viewport.y
    };

    CellPos {
        col: clamped_col,
        row: clamped_row,
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
    use super::{LayoutContext, StateCommand, TuiState, truncate_preview};
    use crate::events::types::CoreEvent;
    use crate::tui::{
        output_surface::{CellPos, OutputViewport},
        ui_action::UiAction,
    };

    fn set_layout(
        state: &mut TuiState,
        input_inner_width: u16,
        max_output_scroll_lines_from_bottom: u16,
        output_viewport: OutputViewport,
    ) {
        state.set_layout_context(LayoutContext::new(
            input_inner_width,
            max_output_scroll_lines_from_bottom,
            output_viewport,
        ));
    }

    #[test]
    fn appends_deltas_and_updates_status() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTextDelta(" world".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);
        assert_eq!(state.status(), "Idle");
        assert_eq!(state.output_log(), "hello world\n");
        assert_eq!(state.input(), "");
        assert!(state.take_dirty());
    }

    #[test]
    fn submit_creates_command_and_echoes_output_log() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Insert('h'));
        state.handle_ui_action(UiAction::Insert('i'));

        let command = state.handle_ui_action(UiAction::Submit);
        assert_eq!(command, StateCommand::Submit("hi".to_string()));
        assert_eq!(state.output_log(), "> hi\n");
        assert_eq!(state.status(), "Running");
        assert_eq!(state.input(), "");
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

        assert_eq!(state.output_log(), "[tool] reading src/main.rs:10-14\n");
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
            state.output_log(),
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

        assert_eq!(state.output_log(), "> hi\n\nhello\n");
    }

    #[test]
    fn inserts_blank_line_between_assistant_and_next_user_message() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);
        state.handle_ui_action(UiAction::Paste("next".to_string()));
        let _ = state.handle_ui_action(UiAction::Submit);

        assert_eq!(state.output_log(), "hello\n\n> next\n");
    }

    #[test]
    fn scroll_input_moves_output_offset_and_clamps_at_zero_on_down() {
        let mut state = TuiState::new();
        set_layout(
            &mut state,
            0,
            u16::MAX,
            OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        );
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
        set_layout(
            &mut state,
            0,
            u16::MAX,
            OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        );
        state.handle_ui_action(UiAction::ScrollUp { lines: 4 });
        state.handle_ui_action(UiAction::Paste("hello".to_string()));

        let command = state.handle_ui_action(UiAction::Submit);

        assert_eq!(command, StateCommand::Submit("hello".to_string()));
        assert_eq!(state.output_scroll_lines_from_bottom(), 0);
    }

    #[test]
    fn cursor_moves_left_and_right_with_clamps() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("hello".to_string()));
        assert_eq!(state.input_cursor_text(), "hello");

        state.handle_ui_action(UiAction::MoveCursorLeft);
        state.handle_ui_action(UiAction::MoveCursorLeft);
        assert_eq!(state.input_cursor_text(), "hel");

        state.handle_ui_action(UiAction::MoveCursorRight);
        assert_eq!(state.input_cursor_text(), "hell");

        for _ in 0..10 {
            state.handle_ui_action(UiAction::MoveCursorLeft);
        }
        assert_eq!(state.input_cursor_text(), "");

        for _ in 0..10 {
            state.handle_ui_action(UiAction::MoveCursorRight);
        }
        assert_eq!(state.input_cursor_text(), "hello");
    }

    #[test]
    fn cursor_moves_up_and_down_across_wrapped_input_rows() {
        let mut state = TuiState::new();
        set_layout(
            &mut state,
            5,
            u16::MAX,
            OutputViewport {
                x: 0,
                y: 0,
                width: 10,
                height: 2,
            },
        );
        state.handle_ui_action(UiAction::Paste("abcdefghij".to_string()));
        let end_len = state.input_cursor_text().len();

        state.handle_ui_action(UiAction::MoveCursorUp);
        let mid_len = state.input_cursor_text().len();
        assert!(mid_len < end_len);

        state.handle_ui_action(UiAction::MoveCursorDown);
        assert_eq!(state.input_cursor_text().len(), end_len);
    }

    #[test]
    fn output_selection_drag_copies_selected_cells() {
        let mut state = TuiState::new();
        set_layout(
            &mut state,
            0,
            u16::MAX,
            OutputViewport {
                x: 10,
                y: 4,
                width: 6,
                height: 3,
            },
        );

        state.handle_ui_action(UiAction::OutputSelectStart { col: 11, row: 4 });
        state.handle_ui_action(UiAction::OutputSelectDrag { col: 13, row: 5 });
        state.handle_ui_action(UiAction::OutputSelectEnd { col: 13, row: 5 });

        assert_eq!(
            state.take_pending_copy_range(),
            Some((CellPos { col: 1, row: 0 }, CellPos { col: 3, row: 1 }))
        );
        assert_eq!(state.take_pending_copy_range(), None);
    }

    #[test]
    fn output_selection_click_without_drag_does_not_copy() {
        let mut state = TuiState::new();
        set_layout(
            &mut state,
            0,
            u16::MAX,
            OutputViewport {
                x: 0,
                y: 0,
                width: 5,
                height: 2,
            },
        );

        state.handle_ui_action(UiAction::OutputSelectStart { col: 2, row: 1 });
        state.handle_ui_action(UiAction::OutputSelectEnd { col: 2, row: 1 });

        assert_eq!(state.take_pending_copy_range(), None);
    }

    #[test]
    fn insert_and_backspace_apply_at_cursor_position() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("ac".to_string()));
        state.handle_ui_action(UiAction::MoveCursorLeft);
        state.handle_ui_action(UiAction::Insert('b'));
        assert_eq!(state.input(), "abc");
        assert_eq!(state.input_cursor_text(), "ab");

        state.handle_ui_action(UiAction::Backspace);
        assert_eq!(state.input(), "ac");
        assert_eq!(state.input_cursor_text(), "a");
    }

    #[test]
    fn line_home_end_and_delete_apply_at_cursor_position() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("hello".to_string()));

        state.handle_ui_action(UiAction::MoveCursorLineStart);
        assert_eq!(state.input_cursor_text(), "");

        state.handle_ui_action(UiAction::Delete);
        assert_eq!(state.input(), "ello");
        assert_eq!(state.input_cursor_text(), "");

        state.handle_ui_action(UiAction::MoveCursorLineEnd);
        assert_eq!(state.input_cursor_text(), "ello");
    }

    #[test]
    fn line_navigation_and_delete_apply_within_current_line() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("ab\ncd\nef".to_string()));

        state.handle_ui_action(UiAction::MoveCursorLineStart);
        assert_eq!(state.input_cursor_text(), "ab\ncd\n");

        state.handle_ui_action(UiAction::DeleteToLineEnd);
        assert_eq!(state.input(), "ab\ncd\n");
        assert_eq!(state.input_cursor_text(), "ab\ncd\n");

        state.handle_ui_action(UiAction::MoveCursorLeft);
        state.handle_ui_action(UiAction::MoveCursorLeft);
        state.handle_ui_action(UiAction::DeleteToLineStart);
        assert_eq!(state.input(), "ab\nd\n");
        assert_eq!(state.input_cursor_text(), "ab\n");

        state.handle_ui_action(UiAction::MoveCursorLineEnd);
        assert_eq!(state.input_cursor_text(), "ab\nd");
    }

    #[test]
    fn word_navigation_and_word_delete_apply_at_cursor_position() {
        let mut state = TuiState::new();
        state.handle_ui_action(UiAction::Paste("hello   world test".to_string()));
        assert_eq!(state.input_cursor_text(), "hello   world test");

        state.handle_ui_action(UiAction::MoveCursorWordLeft);
        assert_eq!(state.input_cursor_text(), "hello   world ");
        state.handle_ui_action(UiAction::MoveCursorWordLeft);
        assert_eq!(state.input_cursor_text(), "hello   ");
        state.handle_ui_action(UiAction::MoveCursorWordRight);
        assert_eq!(state.input_cursor_text(), "hello   world");

        state.handle_ui_action(UiAction::DeleteWordLeft);
        assert_eq!(state.input(), "hello    test");
        assert_eq!(state.input_cursor_text(), "hello   ");
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
        set_layout(
            &mut state,
            0,
            u16::MAX,
            OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        );
        state.handle_ui_action(UiAction::ScrollUp { lines: 100 });
        assert_eq!(state.output_scroll_lines_from_bottom(), 100);

        set_layout(
            &mut state,
            0,
            5,
            OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        );

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
