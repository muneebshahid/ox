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
const OUTPUT_FIXED_ROWS: u16 = 0;

pub(super) struct OutputView {
    pub(super) text: Text<'static>,
    pub(super) plain_lines: Vec<String>,
}

/// Computes output pane height from wrapped output content.
///
/// Inputs:
/// - `lines`: plain output lines used for wrap/height math.
/// - `width`: output viewport width in cells.
/// - `max_height`: maximum rows available for the output pane.
///
/// Output:
/// - Output area height in terminal rows.
pub(super) fn height_rows(lines: &[String], width: u16, max_height: u16) -> u16 {
    viewport::row_height(lines, width, max_height, OUTPUT_FIXED_ROWS)
}

/// Builds the full output payload for this frame.
///
/// Inputs:
/// - `state`: UI state containing output log text.
/// - `meta`: session metadata used by the banner.
///
/// Behavior:
/// - Starts with banner rows.
/// - Appends a blank separator and output log lines when output log is non-empty.
/// - Produces styled and plain representations of the same content.
///
/// Example:
/// - If output log is empty, output contains only banner rows.
/// - If output log is `"hello\nworld"`, output plain lines are:
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
    let output_log = state.output_log();

    if !output_log.is_empty() {
        lines.push(Line::from(String::new()));
        plain_lines.push(String::new());
        for line in output_log.split('\n') {
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
/// - `output`: prebuilt styled/plain output content.
/// - `area`: output pane rectangle from layout.
/// - `scroll_offset_top`: precomputed top-of-buffer offset for paragraph scroll.
///
/// Behavior:
/// - Draws wrapped paragraph content into `area`.
///
/// Output:
/// - No return value; writes widgets into `frame`.
pub(super) fn draw_output(
    frame: &mut Frame<'_>,
    output: &OutputView,
    area: Rect,
    scroll_offset_top: u16,
) {
    let output = Paragraph::new(output.text.clone())
        .scroll((scroll_offset_top, 0))
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
    use super::{BANNER_TOP_PADDING_ROWS, build_output_view, draw_output, height_rows, viewport};
    use crate::{
        events::types::CoreEvent,
        tui::{render::RenderMeta, state::TuiState},
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
    fn output_contains_welcome_metadata_when_output_log_is_empty() {
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
    fn output_appends_output_log_after_blank_separator() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello\nworld".to_string()));

        let view = build_output_view(&state, &test_meta_with_branch(None));
        let separator_index = 6 + BANNER_TOP_PADDING_ROWS;
        assert_eq!(view.plain_lines[separator_index], "");
        assert_eq!(view.plain_lines[separator_index + 1], "hello");
        assert_eq!(view.plain_lines[separator_index + 2], "world");
    }

    #[test]
    fn max_scroll_lines_matches_viewport_scroll_math() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTextDelta(
            (0..40)
                .map(|i| format!("line-{i}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
        let output = build_output_view(&state, &test_meta_with_branch(None));
        let max_scroll = viewport::max_scroll_offset(&output.plain_lines, 20, 5);
        let scroll = viewport::scroll_offset_from_max(max_scroll, max_scroll);

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).expect("create test terminal");
        terminal
            .draw(|frame| {
                draw_output(frame, &output, Rect::new(0, 0, 20, 5), scroll);
            })
            .expect("draw output");
    }

    #[test]
    fn height_rows_respects_available_space() {
        let lines = vec!["one".to_string(); 100];
        assert_eq!(height_rows(&lines, 20, 6), 6);
    }

    #[test]
    fn height_rows_can_be_zero_when_no_space_available() {
        let lines = vec!["one".to_string(); 100];
        assert_eq!(height_rows(&lines, 20, 0), 0);
    }
}
