use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;

const RUNNING_BADGE_TOGGLE_INTERVAL: Duration = Duration::from_millis(250);
const OX_BADGE_BRACKET_COLOR: Color = Color::Yellow;
const OX_BADGE_O_COLOR: Color = Color::Red;
const OX_BADGE_X_COLOR: Color = Color::Cyan;

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
pub(super) fn draw(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
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
