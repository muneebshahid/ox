mod input_pane;
mod output;
mod viewport;

use std::time::Duration;

use super::state::{OutputViewport, TuiState};
use output::{build_output_view, draw_output};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const RUNNING_BADGE_TOGGLE_INTERVAL: Duration = Duration::from_millis(250);
const OX_BADGE_BRACKET_COLOR: Color = Color::Yellow;
const OX_BADGE_O_COLOR: Color = Color::Red;
const OX_BADGE_X_COLOR: Color = Color::Cyan;

pub(super) struct RenderSync {
    pub(super) input_inner_width: u16,
    pub(super) max_output_scroll_lines_from_bottom: u16,
    pub(super) output_viewport: OutputViewport,
    pub(super) output_cells: Vec<Vec<String>>,
}

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
/// - `state`: immutable UI state used for read-only rendering decisions.
/// - `meta`: static banner metadata for this session.
///
/// Behavior:
/// - Builds output content.
/// - Computes the vertical layout split (output, status, input).
/// - Draws output text, optional running status line, input box, and cursor.
///
/// Output:
/// - `RenderSync` values that state owners apply after draw.
pub(super) fn draw(frame: &mut Frame<'_>, state: &TuiState, meta: &RenderMeta) -> RenderSync {
    let area = frame.area();
    let output = build_output_view(state, meta);
    let show_status = state.status_row_visible();
    let status_height_rows = u16::from(show_status);
    let max_input_height_rows = area.height.saturating_sub(status_height_rows);
    let input_height_rows =
        input_pane::height_rows(state.input(), area.width, max_input_height_rows);
    let max_output_height_rows = area
        .height
        .saturating_sub(input_height_rows.saturating_add(status_height_rows));
    let output_height_rows =
        output::height_rows(&output.plain_lines, area.width, max_output_height_rows);

    let [output_area, status_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height_rows),
        Constraint::Length(status_height_rows),
        Constraint::Length(input_height_rows),
        Constraint::Min(0),
    ])
    .areas(area);

    let max_output_scroll_lines_from_bottom =
        viewport::max_scroll_offset(&output.plain_lines, output_area.width, output_area.height);
    let output_scroll_top = viewport::scroll_offset_from_max(
        max_output_scroll_lines_from_bottom,
        state.output_scroll_lines_from_bottom(),
    );
    let output_render_sync = draw_output(
        frame,
        &output,
        output_area,
        output_scroll_top,
        state.output_selection_range(),
    );
    if show_status {
        draw_status(frame, state, status_area);
    }
    let input_inner_width = input_area
        .inner(Margin {
            vertical: 1,
            horizontal: 0,
        })
        .width;
    input_pane::draw_input(frame, state, input_area);
    input_pane::place_input_cursor(frame, state, input_area);

    RenderSync {
        input_inner_width,
        max_output_scroll_lines_from_bottom,
        output_viewport: output_render_sync.viewport,
        output_cells: output_render_sync.cells,
    }
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
    use super::{OX_BADGE_O_COLOR, OX_BADGE_X_COLOR, running_status_line};
    use crate::{events::types::CoreEvent, tui::state::TuiState};
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
    fn status_visibility_tracks_non_idle_state() {
        let mut state = TuiState::new();
        assert!(!state.status_row_visible());

        state.handle_agent_event(CoreEvent::AgentTurnStart);
        assert!(state.status_row_visible());

        state.handle_agent_event(CoreEvent::Error("boom".to_string()));
        assert!(state.status_row_visible());
    }
}
