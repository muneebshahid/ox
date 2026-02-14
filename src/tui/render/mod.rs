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

const INPUT_HEIGHT_ROWS: u16 = 3;
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
    /// Builds the static metadata shown in the output banner.
    ///
    /// Inputs:
    /// - `model`: active model identifier.
    /// - `reasoning`: reasoning effort label.
    /// - `auth_mode`: auth mode label (for example `subscription` or `api`).
    /// - `cwd`: current working directory text shown in the banner.
    /// - `git_branch`: optional git branch label.
    ///
    /// Output:
    /// - A `RenderMeta` value consumed by output rendering.
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

/// Draws one full TUI frame.
///
/// Inputs:
/// - `frame`: current ratatui frame to render into.
/// - `state`: mutable UI state; draw may clamp scroll values based on viewport.
/// - `meta`: static banner metadata for this session.
///
/// Behavior:
/// - Builds output content.
/// - Computes the vertical layout split (output, status, input).
/// - Draws output text, optional running status line, input box, and cursor.
///
/// Output:
/// - No return value; writes widgets into `frame`.
pub fn draw(frame: &mut Frame<'_>, state: &mut TuiState, meta: &RenderMeta) {
    let area = frame.area();
    let output = build_output_view(state, meta);
    let show_status = status_visible(state);
    let status_height_rows = u16::from(show_status);
    let output_height_rows = output_height_rows(&output.plain_lines, area, show_status);

    let [output_area, status_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height_rows),
        Constraint::Length(status_height_rows),
        Constraint::Length(INPUT_HEIGHT_ROWS),
        Constraint::Min(0),
    ])
    .areas(area);

    draw_output(frame, state, &output, output_area);
    if show_status {
        draw_status(frame, state, status_area);
    }
    input::draw_input(frame, state, input_area);
    input::place_input_cursor(frame, state, input_area);
}

/// Computes how many rows to allocate to the output pane.
///
/// Inputs:
/// - `lines`: plain output lines used for wrap/height math.
/// - `area`: full frame area for this draw.
/// - `show_status`: whether the status row will be rendered.
///
/// Behavior:
/// - Reserves fixed space for input and optional status.
/// - Uses viewport wrap math to cap output height to available rows.
///
/// Example:
/// - If `area.height = 20`, input height is `3`, and `show_status = true`,
///   reserved height is `4`, so max output height is `16`.
/// - If wrapped output lines require `30` rows, returned output height is `16`.
/// - If wrapped output lines require `8` rows, returned output height is `8`.
///
/// Output:
/// - Output area height in terminal rows.
fn output_height_rows(lines: &[String], area: Rect, show_status: bool) -> u16 {
    let status_height = u16::from(show_status);
    let reserved_height = INPUT_HEIGHT_ROWS.saturating_add(status_height);
    let max_output_height = area.height.saturating_sub(reserved_height).max(1);
    viewport::clamped_output_height(lines, area.width, max_output_height)
}

/// Draws the status row.
///
/// Inputs:
/// - `frame`: frame to render into.
/// - `state`: UI state containing status and running phase/time.
/// - `area`: layout region for the status row.
///
/// Behavior:
/// - Shows animated running badge line while running.
/// - Otherwise shows idle/error status text.
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

/// Returns whether the status row should be visible.
///
/// Input:
/// - `state`: current UI state.
///
/// Output:
/// - `true` when status is not `Idle`, otherwise `false`.
fn status_visible(state: &TuiState) -> bool {
    state.status() != "Idle"
}

/// Builds the animated running status line: `[OX] <phase> (<seconds>s)`.
///
/// Inputs:
/// - `elapsed`: time since run start.
/// - `phase`: textual phase label such as `Thinking` or `Responding`.
///
/// Behavior:
/// - Alternates bold emphasis between `O` and `X` at a fixed interval.
///
/// Output:
/// - Styled `Line` for the status row.
fn running_status_line(elapsed: Duration, phase: &str) -> Line<'static> {
    let interval_ms = RUNNING_BADGE_TOGGLE_INTERVAL.as_millis().max(1);
    let highlight_o = (elapsed.as_millis() / interval_ms).is_multiple_of(2);
    let elapsed_seconds = elapsed.as_secs();

    let bold = |color| Style::default().fg(color).add_modifier(Modifier::BOLD);
    let plain = |color| Style::default().fg(color);
    let bracket_style = bold(OX_BADGE_BRACKET_COLOR);

    let (o_style, x_style) = if highlight_o {
        (bold(OX_BADGE_O_COLOR), plain(OX_BADGE_X_COLOR))
    } else {
        (plain(OX_BADGE_O_COLOR), bold(OX_BADGE_X_COLOR))
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
    use super::{
        OX_BADGE_O_COLOR, OX_BADGE_X_COLOR, output_height_rows, running_status_line, status_visible,
    };
    use crate::{events::types::CoreEvent, tui::state::TuiState};
    use ratatui::layout::Rect;
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
    }

    #[test]
    fn output_height_rows_respects_reserved_rows_for_input_and_status() {
        let lines = vec!["one".to_string(); 100];
        let area = Rect::new(0, 0, 20, 10);

        let with_status = output_height_rows(&lines, area, true);
        let without_status = output_height_rows(&lines, area, false);

        assert_eq!(with_status, 6);
        assert_eq!(without_status, 7);
    }

    #[test]
    fn status_visibility_tracks_non_idle_state() {
        let mut state = TuiState::new();
        assert!(!status_visible(&state));

        state.handle_agent_event(CoreEvent::AgentTurnStart);
        assert!(status_visible(&state));

        state.handle_agent_event(CoreEvent::Error("boom".to_string()));
        assert!(status_visible(&state));
    }
}
