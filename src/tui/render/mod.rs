mod input;
mod output;
mod viewport;

use std::time::Duration;

use super::state::TuiState;
use output::{build_output_view, draw_output};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const INPUT_HEIGHT: u16 = 3;
const RUNNING_BADGE_TOGGLE_INTERVAL: Duration = Duration::from_millis(250);
const OX_BADGE_BRACKET_COLOR: Color = Color::Yellow;
const OX_BADGE_O_COLOR: Color = Color::Red;
const OX_BADGE_X_COLOR: Color = Color::Cyan;

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
    let bracket_style = Style::default()
        .fg(OX_BADGE_BRACKET_COLOR)
        .add_modifier(Modifier::BOLD);
    let (o_style, x_style) = if highlight_o {
        (
            Style::default()
                .fg(OX_BADGE_O_COLOR)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(OX_BADGE_X_COLOR),
        )
    } else {
        (
            Style::default().fg(OX_BADGE_O_COLOR),
            Style::default()
                .fg(OX_BADGE_X_COLOR)
                .add_modifier(Modifier::BOLD),
        )
    };

    Line::from(vec![
        Span::styled("[", bracket_style),
        Span::styled("O", o_style),
        Span::styled("X", x_style),
        Span::styled("]", bracket_style),
        Span::raw(format!(" {phase} ({elapsed_seconds}s)")),
    ])
}

#[cfg(test)]
mod tests {
    use super::{OX_BADGE_O_COLOR, OX_BADGE_X_COLOR, running_status_line};
    use ratatui::style::Color;
    use std::time::Duration;

    #[test]
    fn running_status_line_keeps_expected_text() {
        let line = running_status_line(Duration::from_secs(3), "Thinking");
        let rendered: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(rendered, "[OX] Thinking (3s)");
    }

    #[test]
    fn running_status_line_colors_ox_badge() {
        let line = running_status_line(Duration::ZERO, "Thinking");

        assert_eq!(line.spans[1].style.fg, Some(OX_BADGE_O_COLOR));
        assert_eq!(line.spans[2].style.fg, Some(OX_BADGE_X_COLOR));
        assert_eq!(line.spans[1].style.fg, Some(Color::Red));
        assert_eq!(line.spans[2].style.fg, Some(Color::Cyan));
    }
}
