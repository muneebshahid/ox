use super::{RenderMeta, viewport};
use crate::tui::state::TuiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

const LOGO_COL_WIDTH: usize = 18;

pub(super) struct OutputView {
    pub(super) text: Text<'static>,
    pub(super) plain_lines: Vec<String>,
}

pub(super) fn build_output_view(state: &TuiState, meta: &RenderMeta) -> OutputView {
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

pub(super) fn draw_output(frame: &mut Frame<'_>, output: &OutputView, area: Rect) {
    let scroll = viewport::scroll_offset(&output.plain_lines, area.width, area.height);
    let output = Paragraph::new(output.text.clone())
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);
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
        ("reasoning", meta.reasoning.as_str()),
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

        let logo_style = logo_colors
            .get(row_idx)
            .map_or_else(Style::default, |color| {
                Style::default().fg(*color).add_modifier(Modifier::BOLD)
            });
        let mut spans = vec![
            Span::styled(logo_text.to_string(), logo_style),
            Span::raw(" ".repeat(padding)),
        ];
        if let Some((label, value)) = meta_row {
            spans.push(Span::styled(format!("{label:<6}"), label_style));
            spans.push(Span::raw(": "));
            spans.push(Span::styled(value.to_string(), value_style));
        }
        lines.push(Line::from(spans));
    }

    (lines, plain_lines)
}

#[cfg(test)]
mod tests {
    use super::build_output_view;
    use crate::{
        events::types::CoreEvent,
        tui::{render::RenderMeta, state::TuiState},
    };

    fn test_meta_with_branch(branch: Option<&str>) -> RenderMeta {
        RenderMeta::new(
            "gpt-5.3-codex".to_string(),
            "medium".to_string(),
            "subscription".to_string(),
            "~/work/ox".to_string(),
            branch.map(std::string::ToString::to_string),
        )
    }

    #[test]
    fn output_contains_welcome_metadata_when_transcript_is_empty() {
        let state = TuiState::new();
        let view = build_output_view(&state, &test_meta_with_branch(None));

        assert_eq!(view.plain_lines.len(), 6);
        assert!(view.plain_lines[0].contains("model"));
        assert!(view.plain_lines[1].contains("reasoning"));
        assert!(view.plain_lines[2].contains("auth"));
        assert!(view.plain_lines[3].contains("cwd"));
    }

    #[test]
    fn output_includes_branch_row_when_available() {
        let state = TuiState::new();
        let view = build_output_view(&state, &test_meta_with_branch(Some("main")));

        assert!(
            view.plain_lines
                .iter()
                .any(|line| line.contains("branch") && line.contains("main"))
        );
    }

    #[test]
    fn output_appends_transcript_after_blank_separator() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello\nworld".to_string()));

        let view = build_output_view(&state, &test_meta_with_branch(None));
        let separator_index = 6;
        assert_eq!(view.plain_lines[separator_index], "");
        assert_eq!(view.plain_lines[separator_index + 1], "hello");
        assert_eq!(view.plain_lines[separator_index + 2], "world");
    }
}
