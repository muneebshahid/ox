use crate::tui::output_surface::CellPos;

pub(super) struct OutputState {
    pub(super) log: String, // Append-only output log shown in the output pane.
    pub(super) scroll_lines_from_bottom: u16, // Manual scroll distance measured from bottom.
    pub(super) reasoning_trace_open: bool, // Whether `[thinking]` trace is currently open.
    pub(super) selection: Option<OutputSelection>, // Active or completed output selection state.
}

impl OutputState {
    pub(super) const fn new() -> Self {
        Self {
            log: String::new(),
            scroll_lines_from_bottom: 0,
            reasoning_trace_open: false,
            selection: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OutputSelection {
    pub(super) anchor: CellPos,
    pub(super) focus: CellPos,
    pub(super) selecting: bool,
    pub(super) pending_copy: bool,
}
