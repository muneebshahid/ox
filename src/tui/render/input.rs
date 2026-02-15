use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Margin, Rect},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

const INPUT_PROMPT_PREFIX: &str = "> ";
const INPUT_BORDER_ROWS: u16 = 2;
const CURSOR_SENTINEL_SYMBOL: &str = "\0";
const INPUT_WRAP: Wrap = Wrap { trim: false };

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
        .wrap(INPUT_WRAP);
    frame.render_widget(input, area);
}

/// Places the terminal cursor at the visual end of the current input text.
///
/// Inputs:
/// - `frame`: frame whose cursor position will be updated.
/// - `state`: UI state containing current typed input.
/// - `input_area`: layout area for the full input widget (including borders).
///
/// Behavior:
/// - Converts outer widget area into inner writable area by removing borders.
/// - Computes wrapped cursor `(col, row)` offset for `> {input}`.
/// - Sets frame cursor position if inner area is non-empty.
pub(super) fn place_input_cursor(frame: &mut Frame<'_>, state: &TuiState, input_area: Rect) {
    let input_inner = input_area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });

    if input_inner.width > 0 && input_inner.height > 0 {
        let (col, row) =
            cursor_offset_with_prompt(state.input(), input_inner.width, input_inner.height);
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
    let content_height = wrapped_content_height(input, width, max_content_height);
    content_height
        .saturating_add(INPUT_BORDER_ROWS)
        .min(max_height)
}

/// Computes cursor offset while accounting for the `> ` prompt prefix.
///
/// Inputs:
/// - `input`: raw user-typed input text.
/// - `width`: inner input area width in cells.
/// - `height`: inner input area height in rows.
///
/// Output:
/// - `(col, row)` cursor position relative to the input inner area.
fn cursor_offset_with_prompt(input: &str, width: u16, height: u16) -> (u16, u16) {
    let display = input_display_text(input);
    cursor_offset(&display, width, height)
}

/// Builds the displayed input text with prompt prefix.
fn input_display_text(input: &str) -> String {
    format!("{INPUT_PROMPT_PREFIX}{input}")
}

fn wrapped_content_height(input: &str, width: u16, max_rows: u16) -> u16 {
    if max_rows == 0 {
        return 0;
    }

    if width == 0 {
        return 1.min(max_rows);
    }

    rendered_row_count(&input_display_text(input), width, max_rows)
}

/// Computes wrapped cursor position for arbitrary text in a bounded rectangle.
///
/// Inputs:
/// - `input`: text to measure.
/// - `width`: available width in cells.
/// - `height`: available height in rows.
///
/// Behavior:
/// - Uses paragraph rendering with wrapping (`trim = false`) to mirror UI behavior.
/// - Finds the last rendered cell and advances cursor by one cell.
/// - Clamps to bottom-right cell when text exceeds visible input area.
///
/// Output:
/// - `(col, row)` position relative to the top-left of the measured area.
fn cursor_offset(input: &str, width: u16, height: u16) -> (u16, u16) {
    last_rendered_cell(input, width, height)
        .map_or((0, 0), |(col, row)| advance_cursor(col, row, width, height))
}

const fn advance_cursor(col: u16, row: u16, width: u16, height: u16) -> (u16, u16) {
    if col + 1 < width {
        return (col + 1, row);
    }
    if row + 1 < height {
        return (0, row + 1);
    }
    (width.saturating_sub(1), height.saturating_sub(1))
}

fn rendered_row_count(input: &str, width: u16, max_rows: u16) -> u16 {
    let Some((_, last_row)) = last_rendered_cell(input, width, max_rows) else {
        return 1.min(max_rows);
    };
    last_row.saturating_add(1)
}

fn last_rendered_cell(input: &str, width: u16, height: u16) -> Option<(u16, u16)> {
    if width == 0 || height == 0 {
        return None;
    }

    let area = Rect::new(0, 0, width, height);
    let mut sentinel = Cell::default();
    sentinel.set_symbol(CURSOR_SENTINEL_SYMBOL);
    let mut scratch = Buffer::filled(area, sentinel);

    Paragraph::new(input.to_string())
        .wrap(INPUT_WRAP)
        .render(area, &mut scratch);

    for row in (0..height).rev() {
        for col in (0..width).rev() {
            if scratch[(col, row)].symbol() != CURSOR_SENTINEL_SYMBOL {
                return Some((col, row));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{cursor_offset, height_rows};

    #[test]
    fn keeps_cursor_on_first_line_when_input_fits() {
        assert_eq!(cursor_offset("hello", 20, 1), (5, 0));
    }

    #[test]
    fn wraps_cursor_when_input_reaches_width() {
        assert_eq!(cursor_offset("hello", 5, 3), (0, 1));
    }

    #[test]
    fn clamps_cursor_to_visible_input_area() {
        assert_eq!(cursor_offset("abcdefghijk", 3, 2), (2, 1));
    }

    #[test]
    fn resets_column_for_newline() {
        assert_eq!(cursor_offset("ab\ncd", 10, 5), (2, 1));
    }

    #[test]
    fn matches_word_wrapping_for_space_separated_text() {
        assert_eq!(cursor_offset("> hello world", 8, 5), (5, 1));
    }

    #[test]
    fn returns_origin_for_zero_sized_input_area() {
        assert_eq!(cursor_offset("hello", 0, 3), (0, 0));
        assert_eq!(cursor_offset("hello", 3, 0), (0, 0));
    }

    #[test]
    fn height_rows_uses_single_content_row_when_input_fits() {
        assert_eq!(height_rows("", 20, 10), 3);
    }

    #[test]
    fn height_rows_accounts_for_wrapping() {
        assert_eq!(height_rows("abcd", 4, 10), 4);
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
