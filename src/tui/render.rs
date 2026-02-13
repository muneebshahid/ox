use super::state::TuiState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const INPUT_HEIGHT: u16 = 3;
const LOGO_COL_WIDTH: usize = 18;

pub struct RenderMeta {
    model: String,
    auth_mode: String,
    cwd: String,
    git_branch: Option<String>,
}

impl RenderMeta {
    pub const fn new(
        model: String,
        auth_mode: String,
        cwd: String,
        git_branch: Option<String>,
    ) -> Self {
        Self {
            model,
            auth_mode,
            cwd,
            git_branch,
        }
    }
}

struct OutputView {
    text: Text<'static>,
    plain_lines: Vec<String>,
}

pub fn draw(frame: &mut Frame<'_>, state: &TuiState, meta: &RenderMeta) {
    let area = frame.area();
    let output = build_output_view(state, meta);
    let show_status = status_visible(state);
    let status_height = u16::from(show_status);
    let reserved_height = INPUT_HEIGHT.saturating_add(status_height);
    let max_output_height = area.height.saturating_sub(reserved_height).max(1);
    let output_height = count_wrapped_lines(&output.plain_lines, area.width).min(max_output_height);

    if show_status {
        let [output_area, status_area, input_area, _rest] = Layout::vertical([
            Constraint::Length(output_height),
            Constraint::Length(1),
            Constraint::Length(INPUT_HEIGHT),
            Constraint::Min(0),
        ])
        .areas(area);
        draw_output(frame, &output, output_area);
        draw_status(frame, state, status_area);
        draw_input(frame, state, input_area);
        place_input_cursor(frame, state, input_area);
        return;
    }

    let [output_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height),
        Constraint::Length(INPUT_HEIGHT),
        Constraint::Min(0),
    ])
    .areas(area);
    draw_output(frame, &output, output_area);
    draw_input(frame, state, input_area);
    place_input_cursor(frame, state, input_area);
}

fn build_output_view(state: &TuiState, meta: &RenderMeta) -> OutputView {
    let (mut lines, mut plain_lines) = welcome_lines(meta);

    if !state.transcript().is_empty() {
        lines.push(Line::from(String::new()));
        plain_lines.push(String::new());
        for line in state.transcript().split('\n') {
            lines.push(Line::from(line.to_string()));
            plain_lines.push(line.to_string());
        }
    }

    OutputView {
        text: Text::from(lines),
        plain_lines,
    }
}

fn welcome_lines(meta: &RenderMeta) -> (Vec<Line<'static>>, Vec<String>) {
    let logo_rows = [
        "██████╗ ██╗  ██╗",
        "██╔═══██╗╚██╗██╔╝",
        "██║   ██║ ╚███╔╝ ",
        "██║   ██║ ██╔██╗ ",
        "╚██████╔╝██╔╝ ██╗",
        " ╚═════╝ ╚═╝  ╚═╝",
    ];
    let logo_colors = [
        Color::Red,
        Color::LightRed,
        Color::Yellow,
        Color::Green,
        Color::Cyan,
        Color::Magenta,
    ];

    let mut meta_rows = vec![
        ("model", meta.model.as_str()),
        ("auth", meta.auth_mode.as_str()),
        ("cwd", meta.cwd.as_str()),
    ];
    if let Some(branch) = meta.git_branch.as_deref() {
        meta_rows.push(("branch", branch));
    }

    let row_count = logo_rows.len().max(meta_rows.len());
    let mut lines = Vec::with_capacity(row_count);
    let mut plain_lines = Vec::with_capacity(row_count);
    let label_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::Gray);

    for row_idx in 0..row_count {
        let logo_text = logo_rows.get(row_idx).copied().unwrap_or("");
        let logo_width = logo_text.chars().count();
        let padding = LOGO_COL_WIDTH.saturating_sub(logo_width).saturating_add(3);
        let meta_row = meta_rows.get(row_idx).copied();
        let meta_text =
            meta_row.map_or_else(String::new, |(label, value)| format!("{label:<6}: {value}"));
        let plain_line = format!("{logo_text}{}{}", " ".repeat(padding), meta_text);
        plain_lines.push(plain_line);

        let mut spans: Vec<Span<'static>> = Vec::new();
        if let Some(color) = logo_colors.get(row_idx) {
            let style = Style::default().fg(*color).add_modifier(Modifier::BOLD);
            spans.push(Span::styled(logo_text.to_string(), style));
        } else {
            spans.push(Span::raw(logo_text.to_string()));
        }
        spans.push(Span::raw(" ".repeat(padding)));
        if let Some((label, value)) = meta_row {
            spans.push(Span::styled(format!("{label:<6}"), label_style));
            spans.push(Span::raw(": "));
            spans.push(Span::styled(value.to_string(), value_style));
        }
        lines.push(Line::from(spans));
    }

    (lines, plain_lines)
}

fn draw_output(frame: &mut Frame<'_>, output: &OutputView, area: Rect) {
    let scroll = output_scroll_offset(&output.plain_lines, area.width, area.height);
    let output = Paragraph::new(output.text.clone())
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);
}

fn draw_status(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let status = Paragraph::new(state.status());
    frame.render_widget(status, area);
}

fn draw_input(frame: &mut Frame<'_>, state: &TuiState, area: Rect) {
    let prompt = format!("> {}", state.input());
    let input = Paragraph::new(prompt)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, area);
}

fn place_input_cursor(frame: &mut Frame<'_>, state: &TuiState, input_area: Rect) {
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

fn status_visible(state: &TuiState) -> bool {
    state.status() != "Idle"
}

fn count_wrapped_lines(lines: &[String], width: u16) -> u16 {
    if width == 0 {
        return 1;
    }

    lines
        .iter()
        .map(|line| {
            let chars = line.chars().count().max(1);
            let width = usize::from(width);
            let wrapped = chars.saturating_add(width.saturating_sub(1)) / width;
            u16::try_from(wrapped).unwrap_or(u16::MAX)
        })
        .fold(0_u16, u16::saturating_add)
        .max(1)
}

fn output_scroll_offset(lines: &[String], width: u16, height: u16) -> u16 {
    if height == 0 {
        return 0;
    }
    let total_lines = count_wrapped_lines(lines, width);
    total_lines.saturating_sub(height)
}

fn cursor_offset_with_prompt(input: &str, width: u16, height: u16) -> (u16, u16) {
    let display = format!("> {input}");
    cursor_offset(&display, width, height)
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
