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

const OUTPUT_MIN_HEIGHT_ROWS: u16 = 1;
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
    let input_height_rows = input_height_rows(state, area, show_status);
    let output_height_rows =
        output_height_rows(&output.plain_lines, area, show_status, input_height_rows);

    let [output_area, status_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height_rows),
        Constraint::Length(status_height_rows),
        Constraint::Length(input_height_rows),
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

/// Computes how many rows to allocate to the input pane.
///
/// Inputs:
/// - `state`: current UI state containing typed input.
/// - `area`: full frame area for this draw.
/// - `show_status`: whether the status row will be rendered.
///
/// Behavior:
/// - Leaves one output row when there is enough vertical space.
/// - Delegates wrapped input sizing to `input::clamped_height_rows`.
///
/// Output:
/// - Input area height in terminal rows.
fn input_height_rows(state: &TuiState, area: Rect, show_status: bool) -> u16 {
    let status_height = u16::from(show_status);
    let available_height = area.height.saturating_sub(status_height);
    if available_height == 0 {
        return 0;
    }

    let max_input_height = if available_height > OUTPUT_MIN_HEIGHT_ROWS {
        available_height.saturating_sub(OUTPUT_MIN_HEIGHT_ROWS)
    } else {
        available_height
    };

    input::clamped_height_rows(state.input(), area.width, max_input_height)
}

/// Computes how many rows to allocate to the output pane.
///
/// Inputs:
/// - `lines`: plain output lines used for wrap/height math.
/// - `area`: full frame area for this draw.
/// - `show_status`: whether the status row will be rendered.
/// - `input_height_rows`: rows already reserved for the input pane.
///
/// Behavior:
/// - Reserves input and optional status rows first.
/// - Uses viewport wrap math to cap output height to remaining rows.
///
/// Output:
/// - Output area height in terminal rows.
fn output_height_rows(
    lines: &[String],
    area: Rect,
    show_status: bool,
    input_height_rows: u16,
) -> u16 {
    let status_height = u16::from(show_status);
    let reserved_height = input_height_rows.saturating_add(status_height);
    let max_output_height = area.height.saturating_sub(reserved_height);
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
        OX_BADGE_O_COLOR, OX_BADGE_X_COLOR, input_height_rows, output_height_rows,
        running_status_line, status_visible,
    };
    use crate::{
        events::types::CoreEvent,
        tui::{state::TuiState, ui_action::UiAction},
    };
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

        let with_status = output_height_rows(&lines, area, true, 3);
        let without_status = output_height_rows(&lines, area, false, 3);

        assert_eq!(with_status, 6);
        assert_eq!(without_status, 7);
    }

    #[test]
    fn input_height_rows_grows_when_input_wraps() {
        let mut state = TuiState::new();
        let _ = state.handle_ui_action(UiAction::Paste("abcdefghij".to_string()));

        let height = input_height_rows(&state, Rect::new(0, 0, 8, 20), false);

        assert_eq!(height, 4);
    }

    #[test]
    fn input_height_rows_preserves_output_space_when_possible() {
        let mut state = TuiState::new();
        let _ = state.handle_ui_action(UiAction::Paste("x".repeat(300)));

        let height = input_height_rows(&state, Rect::new(0, 0, 10, 6), false);

        assert_eq!(height, 5);
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
