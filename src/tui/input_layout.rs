use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    widgets::{Paragraph, Widget, Wrap},
};

pub(super) const INPUT_PROMPT_PREFIX: &str = "> ";
pub(super) const INPUT_WRAP: Wrap = Wrap { trim: false };

const CURSOR_SENTINEL_SYMBOL: &str = "\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorPosition {
    pub(super) byte: usize,
    pub(super) col: u16,
    pub(super) row: u16,
}

pub(super) fn input_display_text(input: &str) -> String {
    format!("{INPUT_PROMPT_PREFIX}{input}")
}

pub(super) fn wrapped_content_height_with_prompt(input: &str, width: u16, max_rows: u16) -> u16 {
    if max_rows == 0 {
        return 0;
    }

    if width == 0 {
        return 1.min(max_rows);
    }

    rendered_row_count(&input_display_text(input), width, max_rows)
}

pub(super) fn cursor_offset_with_prompt(input: &str, width: u16, height: u16) -> (u16, u16) {
    let display = input_display_text(input);
    cursor_offset(&display, width, height)
}

pub(super) fn cursor_positions_with_prompt(input: &str, width: u16) -> Vec<CursorPosition> {
    if width == 0 {
        return vec![CursorPosition {
            byte: 0,
            col: 0,
            row: 0,
        }];
    }

    let height = measurement_height_with_prompt(input, width);

    cursor_boundaries(input)
        .into_iter()
        .map(|byte| {
            let (col, row) = cursor_offset_with_prompt(&input[..byte], width, height);
            CursorPosition { byte, col, row }
        })
        .collect()
}

fn measurement_height_with_prompt(input: &str, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }

    let prompt_chars = INPUT_PROMPT_PREFIX.chars().count();
    let chars = input.chars().count();
    let newlines = input.chars().filter(|ch| *ch == '\n').count();
    let spaces = input
        .chars()
        .filter(|ch| ch.is_whitespace() && *ch != '\n')
        .count();

    let wrapped_chars = prompt_chars
        .saturating_add(chars)
        .div_ceil(usize::from(width));
    let upper_bound = wrapped_chars
        .saturating_add(newlines)
        .saturating_add(spaces)
        .saturating_add(2)
        .max(1);

    u16::try_from(upper_bound).unwrap_or(u16::MAX)
}

fn cursor_boundaries(input: &str) -> Vec<usize> {
    let mut boundaries: Vec<usize> = input.char_indices().map(|(idx, _)| idx).collect();
    if boundaries.first().is_none_or(|idx| *idx != 0) {
        boundaries.insert(0, 0);
    }
    boundaries.push(input.len());
    boundaries.dedup();
    boundaries
}

pub(super) fn cursor_offset(input: &str, width: u16, height: u16) -> (u16, u16) {
    let trailing_newlines = trailing_newline_count(input);
    if trailing_newlines > 0 {
        let base_row = last_rendered_cell(input, width, height).map_or(0, |(_, row)| row);
        let row = base_row.saturating_add(trailing_newlines);
        return (0, row.min(height.saturating_sub(1)));
    }

    last_rendered_cell(input, width, height)
        .map_or((0, 0), |(col, row)| advance_cursor(col, row, width, height))
}

fn rendered_row_count(input: &str, width: u16, max_rows: u16) -> u16 {
    let base_rows = last_rendered_cell(input, width, max_rows)
        .map_or(1_u16, |(_, last_row)| last_row.saturating_add(1));
    base_rows
        .saturating_add(trailing_newline_count(input))
        .min(max_rows)
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

fn trailing_newline_count(input: &str) -> u16 {
    let trailing = input.chars().rev().take_while(|ch| *ch == '\n').count();
    u16::try_from(trailing).unwrap_or(u16::MAX)
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
    use super::{cursor_offset, cursor_positions_with_prompt, wrapped_content_height_with_prompt};

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
    fn moves_cursor_to_next_line_for_trailing_newline() {
        assert_eq!(cursor_offset("hello\n", 20, 5), (0, 1));
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
    fn content_height_reserves_extra_row_for_trailing_newline() {
        assert_eq!(wrapped_content_height_with_prompt("hello\n", 20, 10), 2);
    }

    #[test]
    fn returns_cursor_positions_for_all_char_boundaries() {
        let positions = cursor_positions_with_prompt("ab", 10);
        let bytes: Vec<usize> = positions.iter().map(|pos| pos.byte).collect();
        assert_eq!(bytes, vec![0, 1, 2]);
    }
}
