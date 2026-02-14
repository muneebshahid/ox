use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::{Block, Borders, Paragraph},
};

const INPUT_PROMPT_PREFIX: &str = "> ";
const INPUT_MIN_HEIGHT_ROWS: u16 = 3;
const INPUT_BORDER_ROWS: u16 = 2;

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
/// - Scrolls vertically to keep the end of input visible when wrapped text
///   exceeds visible input rows.
pub(super) fn draw_input(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let display = input_with_prompt(state.input());
    let input_inner = area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    let scroll = input_scroll_offset(&display, input_inner.width, input_inner.height);
    let rendered = hard_wrap_for_display(&display, input_inner.width);

    let input = Paragraph::new(rendered)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
        .scroll((scroll, 0));
    frame.render_widget(input, area);
}

/// Computes input pane height clamped to a caller-provided maximum.
///
/// Inputs:
/// - `input`: raw user input text.
/// - `width`: full input widget width in cells.
/// - `max_height`: maximum allowed input widget height in rows.
///
/// Behavior:
/// - Starts from a minimum visual input height (prompt row + borders).
/// - Grows height as wrapped input text needs more rows.
/// - Clamps to `max_height`.
///
/// Output:
/// - Input widget height in rows, including borders.
pub(super) fn clamped_height_rows(input: &str, width: u16, max_height: u16) -> u16 {
    if max_height == 0 {
        return 0;
    }

    let required = required_height_rows(input, width);
    let min_height = INPUT_MIN_HEIGHT_ROWS.min(max_height);
    required.max(min_height).min(max_height)
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
/// - Applies input vertical scroll offset so cursor tracks the visible viewport.
/// - Sets frame cursor position if inner area is non-empty.
pub(super) fn place_input_cursor(frame: &mut Frame<'_>, state: &TuiState, input_area: Rect) {
    let input_inner = input_area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });

    if input_inner.width > 0 && input_inner.height > 0 {
        let display = input_with_prompt(state.input());
        let scroll = input_scroll_offset(&display, input_inner.width, input_inner.height);
        let (col, absolute_row) = cursor_offset(&display, input_inner.width, u16::MAX);
        let row = absolute_row
            .saturating_sub(scroll)
            .min(input_inner.height.saturating_sub(1));

        frame.set_cursor_position((input_inner.x + col, input_inner.y + row));
    }
}

fn input_with_prompt(input: &str) -> String {
    format!("{INPUT_PROMPT_PREFIX}{input}")
}

/// Converts logical input text into explicit newline-wrapped display text.
///
/// This keeps visual wrapping behavior consistent with cursor and height math by
/// using character-based wrapping instead of widget word-wrapping.
fn hard_wrap_for_display(input: &str, width: u16) -> String {
    if width == 0 || input.is_empty() {
        return input.to_string();
    }

    let mut rendered = String::with_capacity(input.len());
    let mut col = 0_u16;

    for ch in input.chars() {
        if ch == '\n' {
            rendered.push('\n');
            col = 0;
            continue;
        }

        rendered.push(ch);
        col = col.saturating_add(1);
        if col >= width {
            rendered.push('\n');
            col = 0;
        }
    }

    rendered
}

fn required_height_rows(input: &str, width: u16) -> u16 {
    wrapped_row_count(&input_with_prompt(input), width).saturating_add(INPUT_BORDER_ROWS)
}

/// Computes vertical scroll needed to keep the end of wrapped input visible.
fn input_scroll_offset(input: &str, width: u16, height: u16) -> u16 {
    if height == 0 {
        return 0;
    }

    wrapped_row_count(input, width).saturating_sub(height)
}

/// Counts wrapped visual rows for arbitrary text in an input-width viewport.
fn wrapped_row_count(input: &str, width: u16) -> u16 {
    if width == 0 || input.is_empty() {
        return 1;
    }

    let mut col = 0_u16;
    let mut rows = 1_u16;

    for ch in input.chars() {
        if ch == '\n' {
            col = 0;
            rows = rows.saturating_add(1);
            continue;
        }

        col = col.saturating_add(1);
        if col >= width {
            col = 0;
            rows = rows.saturating_add(1);
        }
    }

    rows
}

/// Computes wrapped cursor position for arbitrary text in a bounded rectangle.
///
/// Inputs:
/// - `input`: text to measure.
/// - `width`: available width in cells.
/// - `height`: available height in rows.
///
/// Behavior:
/// - Advances by one column per character.
/// - Wraps to next row when column reaches `width`.
/// - Handles explicit newline characters by resetting column and advancing row.
/// - Clamps to bottom-right cell when text exceeds visible input area.
///
/// Output:
/// - `(col, row)` position relative to the top-left of the measured area.
fn cursor_offset(input: &str, width: u16, height: u16) -> (u16, u16) {
    if width == 0 || height == 0 {
        return (0, 0);
    }

    let mut col = 0_u16;
    let mut row = 0_u16;

    for ch in input.chars() {
        if ch == '\n' {
            col = 0;
            row = row.saturating_add(1);
            continue;
        }

        col = col.saturating_add(1);
        if col >= width {
            col = 0;
            row = row.saturating_add(1);
        }
    }

    if row >= height {
        return (width.saturating_sub(1), height.saturating_sub(1));
    }

    (col, row)
}

#[cfg(test)]
mod tests {
    use super::{
        clamped_height_rows, cursor_offset, hard_wrap_for_display, input_scroll_offset,
        wrapped_row_count,
    };

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
    fn returns_origin_for_zero_sized_input_area() {
        assert_eq!(cursor_offset("hello", 0, 3), (0, 0));
        assert_eq!(cursor_offset("hello", 3, 0), (0, 0));
    }

    #[test]
    fn wrapped_rows_counts_newlines_and_soft_wraps() {
        assert_eq!(wrapped_row_count("ab\ncd", 2), 4);
    }

    #[test]
    fn input_height_grows_when_prompt_wraps() {
        assert_eq!(clamped_height_rows("abcdefghij", 8, 10), 4);
    }

    #[test]
    fn input_height_clamps_to_maximum() {
        let long = "x".repeat(200);
        assert_eq!(clamped_height_rows(&long, 10, 5), 5);
    }

    #[test]
    fn input_scroll_follows_bottom_of_wrapped_content() {
        let display = format!("> {}", "x".repeat(20));
        assert_eq!(input_scroll_offset(&display, 5, 3), 2);
    }

    #[test]
    fn hard_wrap_uses_character_wrapping_even_with_spaces() {
        assert_eq!(hard_wrap_for_display("> hello tffff", 8), "> hello \ntffff");
    }
}
