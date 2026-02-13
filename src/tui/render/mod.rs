mod input;
mod output;
mod viewport;

use super::state::TuiState;
use output::{build_output_view, draw_output};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::Paragraph,
};

const INPUT_HEIGHT: u16 = 3;

pub struct RenderMeta {
    model: String,
    reasoning: String,
    auth_mode: String,
    cwd: String,
    git_branch: Option<String>,
}

impl RenderMeta {
    pub const fn new(
        model: String,
        reasoning: String,
        auth_mode: String,
        cwd: String,
        git_branch: Option<String>,
    ) -> Self {
        Self {
            model,
            reasoning,
            auth_mode,
            cwd,
            git_branch,
        }
    }
}

pub fn draw(frame: &mut Frame<'_>, state: &TuiState, meta: &RenderMeta) {
    let area = frame.area();
    let output = build_output_view(state, meta);
    let show_status = status_visible(state);
    let status_height = u16::from(show_status);
    let reserved_height = INPUT_HEIGHT.saturating_add(status_height);
    let max_output_height = area.height.saturating_sub(reserved_height).max(1);
    let output_height =
        viewport::clamped_output_height(&output.plain_lines, area.width, max_output_height);

    let [output_area, status_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height),
        Constraint::Length(status_height),
        Constraint::Length(INPUT_HEIGHT),
        Constraint::Min(0),
    ])
    .areas(area);

    draw_output(frame, &output, output_area);
    if show_status {
        draw_status(frame, state, status_area);
    }
    input::draw_input(frame, state, input_area);
    input::place_input_cursor(frame, state, input_area);
}

fn draw_status(frame: &mut Frame<'_>, state: &TuiState, area: ratatui::layout::Rect) {
    let status = Paragraph::new(state.status());
    frame.render_widget(status, area);
}

fn status_visible(state: &TuiState) -> bool {
    state.status() != "Idle"
}
