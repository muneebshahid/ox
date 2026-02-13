mod input;
mod output;
mod viewport;

use std::time::Duration;

use super::state::TuiState;
use output::{build_output_view, draw_output};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const INPUT_HEIGHT: u16 = 3;
const RUNNING_BADGE_TOGGLE_INTERVAL: Duration = Duration::from_millis(250);

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
    let output_height = output_height(&output.plain_lines, area, show_status);

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

fn output_height(lines: &[String], area: Rect, show_status: bool) -> u16 {
    let status_height = u16::from(show_status);
    let reserved_height = INPUT_HEIGHT.saturating_add(status_height);
    let max_output_height = area.height.saturating_sub(reserved_height).max(1);
    viewport::clamped_output_height(lines, area.width, max_output_height)
}

fn draw_status(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let status = if state.status_is_running() {
        Paragraph::new(running_status_line(
            state.running_elapsed(),
            &state.running_phase_label(),
        ))
    } else {
        Paragraph::new(state.status())
    };
    frame.render_widget(status, area);
}

fn status_visible(state: &TuiState) -> bool {
    state.status() != "Idle"
}

fn running_status_line(elapsed: Duration, phase: &str) -> Line<'static> {
    let interval_ms = RUNNING_BADGE_TOGGLE_INTERVAL.as_millis().max(1);
    let highlight_o = (elapsed.as_millis() / interval_ms).is_multiple_of(2);
    let elapsed_seconds = elapsed.as_secs();
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let (o_style, x_style) = if highlight_o {
        (bold, Style::default())
    } else {
        (Style::default(), bold)
    };

    Line::from(vec![
        Span::raw("["),
        Span::styled("O", o_style),
        Span::styled("X", x_style),
        Span::raw(format!("] {phase} ({elapsed_seconds}s)")),
    ])
}
