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
const BANNER_LEFT_PADDING: usize = 2;
const BANNER_TOP_PADDING_ROWS: usize = 1;

pub(super) struct OutputView {
    pub(super) text: Text<'static>,
    pub(super) plain_lines: Vec<String>,
}

/// Builds the full output payload for this frame.
///
/// Inputs:
/// - `state`: UI state containing transcript text.
/// - `meta`: session metadata used by the banner.
///
/// Behavior:
/// - Starts with banner rows.
/// - Appends a blank separator and transcript lines when transcript is non-empty.
/// - Produces styled and plain representations of the same content.
///
/// Example:
/// - If transcript is empty, output contains only banner rows.
/// - If transcript is `"hello\nworld"`, output plain lines are:
///   - banner rows
///   - `""` (separator)
///   - `"hello"`
///   - `"world"`
/// - `OutputView.text` contains the same logical rows as styled `Line` values
///   for rendering, while `OutputView.plain_lines` stores the unstyled strings
///   used by wrap/scroll math.
///
/// Output:
/// - `OutputView` with:
///   - `text`: styled lines for rendering.
///   - `plain_lines`: unstyled lines for wrap/scroll computations.
pub(super) fn build_output_view(state: &TuiState, meta: &RenderMeta) -> OutputView {
    let (mut lines, mut plain_lines) = build_banner_lines(meta);

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

/// Renders the output pane with scroll state applied.
///
/// Inputs:
/// - `frame`: frame to render into.
/// - `state`: mutable state containing manual scroll position.
/// - `output`: prebuilt styled/plain output content.
/// - `area`: output pane rectangle from layout.
///
/// Behavior:
/// - Computes current maximum scroll from wrapped content and viewport size.
/// - Clamps state scroll to that max to avoid overscroll debt.
/// - Converts "lines from bottom" into ratatui scroll offset.
/// - Draws wrapped paragraph content into `area`.
///
/// Output:
/// - No return value; writes widgets into `frame`.
pub(super) fn draw_output(
    frame: &mut Frame<'_>,
    state: &mut TuiState,
    output: &OutputView,
    area: Rect,
) {
    let max_scroll = viewport::max_scroll_offset(&output.plain_lines, area.width, area.height);
    state.clamp_output_scroll_lines_from_bottom(max_scroll);
    let scroll = viewport::scroll_offset(
        &output.plain_lines,
        area.width,
        area.height,
        state.output_scroll_lines_from_bottom(),
    );
    let output = Paragraph::new(output.text.clone())
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);
}

/// Builds the fixed banner lines shown at the top of the output area.
///
/// Inputs:
/// - `meta`: model/auth/cwd/branch information for this session.
///
/// Behavior:
/// - Composes an ASCII logo column with color styling.
/// - Aligns metadata rows to the right of the logo.
/// - Produces both styled lines and plain text lines with identical layout.
///
/// Output:
/// - Tuple of `(styled_lines, plain_lines)` used by `build_output_view`.
fn build_banner_lines(meta: &RenderMeta) -> (Vec<Line<'static>>, Vec<String>) {
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
    let mut lines = Vec::with_capacity(row_count + BANNER_TOP_PADDING_ROWS);
    let mut plain_lines = Vec::with_capacity(row_count + BANNER_TOP_PADDING_ROWS);
    let label_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::Gray);

    for _ in 0..BANNER_TOP_PADDING_ROWS {
        lines.push(Line::from(String::new()));
        plain_lines.push(String::new());
    }

    for row_idx in 0..row_count {
        let logo_text = logo_rows.get(row_idx).copied().unwrap_or("");
        let logo_width = logo_text.chars().count();
        let padding = LOGO_COL_WIDTH.saturating_sub(logo_width).saturating_add(3);
        let meta_row = meta_rows.get(row_idx).copied();
        let meta_text =
            meta_row.map_or_else(String::new, |(label, value)| format!("{label:<6}: {value}"));
        let plain_line = format!(
            "{}{logo_text}{}{}",
            " ".repeat(BANNER_LEFT_PADDING),
            " ".repeat(padding),
            meta_text
        );
        plain_lines.push(plain_line);

        let logo_style = logo_colors
            .get(row_idx)
            .map_or_else(Style::default, |color| {
                Style::default().fg(*color).add_modifier(Modifier::BOLD)
            });
        let mut spans = vec![
            Span::raw(" ".repeat(BANNER_LEFT_PADDING)),
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
    use super::{BANNER_TOP_PADDING_ROWS, build_output_view, draw_output, viewport};
    use crate::{
        events::types::CoreEvent,
        tui::{action::UiAction, render::RenderMeta, state::TuiState},
    };
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

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

        assert_eq!(view.plain_lines.len(), 6 + BANNER_TOP_PADDING_ROWS);
        assert!(view.plain_lines[BANNER_TOP_PADDING_ROWS].contains("model"));
        assert!(view.plain_lines[1 + BANNER_TOP_PADDING_ROWS].contains("reasoning"));
        assert!(view.plain_lines[2 + BANNER_TOP_PADDING_ROWS].contains("auth"));
        assert!(view.plain_lines[3 + BANNER_TOP_PADDING_ROWS].contains("cwd"));
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
        let separator_index = 6 + BANNER_TOP_PADDING_ROWS;
        assert_eq!(view.plain_lines[separator_index], "");
        assert_eq!(view.plain_lines[separator_index + 1], "hello");
        assert_eq!(view.plain_lines[separator_index + 2], "world");
    }

    #[test]
    fn draw_output_clamps_manual_scroll_to_current_max() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTextDelta(
            (0..40)
                .map(|i| format!("line-{i}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
        let _ = state.handle_ui_action(UiAction::ScrollUp { lines: 500 });
        assert!(state.output_scroll_lines_from_bottom() > 0);

        let output = build_output_view(&state, &test_meta_with_branch(None));
        let expected_max = viewport::max_scroll_offset(&output.plain_lines, 20, 5);

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).expect("create test terminal");
        terminal
            .draw(|frame| {
                draw_output(frame, &mut state, &output, Rect::new(0, 0, 20, 5));
            })
            .expect("draw output");

        assert_eq!(state.output_scroll_lines_from_bottom(), expected_max);
    }
}
