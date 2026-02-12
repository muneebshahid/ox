use super::state::TuiState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw(frame: &mut Frame<'_>, state: &TuiState) {
    let [output_area, status_area, input_area] = split_layout(frame.area());
    draw_output(frame, state, output_area);
    draw_status(frame, state, status_area);
    draw_input(frame, state, input_area);
    place_input_cursor(frame, state, input_area);
}

fn split_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .areas(area)
}

fn draw_output(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let output = Paragraph::new(state.transcript())
        .block(Block::default().title("Output").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);
}

fn draw_status(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let status = Paragraph::new(state.status())
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn draw_input(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let input = Paragraph::new(state.input())
        .block(
            Block::default()
                .title("Input (Enter submit, Esc quit)")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input, area);
}

fn place_input_cursor(frame: &mut Frame<'_>, state: &TuiState, input_area: Rect) {
    let input_inner = input_area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if input_inner.width > 0 && input_inner.height > 0 {
        let (col, row) = cursor_offset(state.input(), input_inner.width, input_inner.height);
        frame.set_cursor_position((input_inner.x + col, input_inner.y + row));
    }
}

fn cursor_offset(input: &str, width: u16, height: u16) -> (u16, u16) {
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
}
