use crate::tui::input_metrics;
use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::{Block, Borders, Paragraph},
};

const INPUT_BORDER_ROWS: u16 = 2;

fn input_display_text(input: &str) -> String {
    format!("{}{input}", input_metrics::INPUT_PROMPT_PREFIX)
}

fn wrapped_content_height_with_prompt(input: &str, width: u16, max_rows: u16) -> u16 {
    if max_rows == 0 {
        return 0;
    }

    if width == 0 {
        return 1.min(max_rows);
    }

    input_metrics::rendered_row_count(&input_display_text(input), width, max_rows)
}

/// Draws the input pane (prompt + typed text) at the bottom of the screen.
///
/// Inputs:
/// - `frame`: frame to render into.
/// - `state`: UI state containing current typed input.
/// - `area`: rectangle allocated to the input widget.
///
/// Behavior:
/// - Prefixes input with `> ` prompt.
/// - Renders top and bottom borders.
/// - Enables wrapping without trimming trailing spaces.
pub(super) fn draw_input(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let input = Paragraph::new(input_display_text(state.input()))
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
        .wrap(input_metrics::INPUT_WRAP);
    frame.render_widget(input, area);
}

/// Places the terminal cursor at the visual input cursor position.
///
/// Inputs:
/// - `frame`: frame whose cursor position will be updated.
/// - `state`: UI state containing current typed input.
/// - `input_area`: layout area for the full input widget (including borders).
///
/// Behavior:
/// - Converts outer widget area into inner writable area by removing borders.
/// - Computes wrapped cursor `(col, row)` offset for `> {input[..cursor]}`.
/// - Sets frame cursor position if inner area is non-empty.
pub(super) fn place_input_cursor(frame: &mut Frame<'_>, state: &TuiState, input_area: Rect) {
    let input_inner = input_area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });

    if input_inner.width > 0 && input_inner.height > 0 {
        let (col, row) = input_metrics::cursor_offset(
            &input_display_text(state.input_cursor_text()),
            input_inner.width,
            input_inner.height,
        );
        frame.set_cursor_position((input_inner.x + col, input_inner.y + row));
    }
}

/// Computes desired input widget height (including borders).
///
/// Inputs:
/// - `input`: raw user-typed input text.
/// - `width`: full input widget width in cells.
/// - `max_height`: maximum rows available for the input widget.
///
/// Behavior:
/// - Measures wrapped rows for displayed input (`> {input}`).
/// - Adds top and bottom border rows.
/// - Clamps to `max_height`.
///
/// Output:
/// - Input widget height in terminal rows.
pub(super) fn height_rows(input: &str, width: u16, max_height: u16) -> u16 {
    if max_height == 0 {
        return 0;
    }

    let max_content_height = max_height.saturating_sub(INPUT_BORDER_ROWS);
    let content_height = wrapped_content_height_with_prompt(input, width, max_content_height);
    content_height
        .saturating_add(INPUT_BORDER_ROWS)
        .min(max_height)
}

#[cfg(test)]
mod tests {
    use super::height_rows;

    #[test]
    fn height_rows_uses_single_content_row_when_input_fits() {
        assert_eq!(height_rows("", 20, 10), 3);
    }

    #[test]
    fn height_rows_accounts_for_wrapping() {
        assert_eq!(height_rows("abcd", 4, 10), 4);
    }

    #[test]
    fn height_rows_reserves_extra_row_for_trailing_newline() {
        assert_eq!(height_rows("hello\n", 20, 10), 4);
    }

    #[test]
    fn height_rows_clamps_to_max_height() {
        assert_eq!(height_rows("abcdefghij", 2, 4), 4);
    }

    #[test]
    fn height_rows_returns_zero_for_zero_max_height() {
        assert_eq!(height_rows("hello", 10, 0), 0);
    }
}
