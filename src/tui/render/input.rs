use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub(super) fn draw_input(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let prompt = format!("> {}", state.input());
    let input = Paragraph::new(prompt)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, area);
}

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

fn cursor_offset_with_prompt(input: &str, width: u16, height: u16) -> (u16, u16) {
    let display = format!("> {input}");
    cursor_offset(&display, width, height)
}

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
    use super::cursor_offset;

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
}
