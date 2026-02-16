use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    widgets::{Paragraph, Widget, Wrap},
};

pub(in crate::tui) const INPUT_PROMPT_PREFIX: &str = "> ";
pub(in crate::tui) const INPUT_WRAP: Wrap = Wrap { trim: false };

const CURSOR_SENTINEL_SYMBOL: &str = "\0";

pub(in crate::tui) fn cursor_offset(input: &str, width: u16, height: u16) -> (u16, u16) {
    let trailing_newlines = trailing_newline_count(input);
    if trailing_newlines > 0 {
        let base_row = last_rendered_cell(input, width, height).map_or(0, |(_, row)| row);
        let row = base_row.saturating_add(trailing_newlines);
        return (0, row.min(height.saturating_sub(1)));
    }

    last_rendered_cell(input, width, height)
        .map_or((0, 0), |(col, row)| advance_cursor(col, row, width, height))
}

pub(in crate::tui) fn rendered_row_count(input: &str, width: u16, max_rows: u16) -> u16 {
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
