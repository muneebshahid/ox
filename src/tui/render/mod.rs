mod input_pane;
mod output;
mod status_pane;
mod viewport;

use super::{
    output_surface::{OutputRenderSnapshot, OutputViewport},
    state::{LayoutContext, TuiState},
};
use output::{build_output_view, draw_output};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
};

struct RenderPlan {
    output: output::OutputView,
    output_area: Rect,
    status_area: Rect,
    input_area: Rect,
    output_scroll_top: u16,
    max_output_scroll_lines_from_bottom: u16,
    input_inner_width: u16,
}

pub struct RenderMeta {
    model: String,
    reasoning: String,
    auth_mode: String,
    cwd: String,
    git_branch: Option<String>,
}

impl RenderMeta {
    /// Builds the static metadata shown in the output banner.
    ///
    /// Inputs:
    /// - `model`: active model identifier.
    /// - `reasoning`: reasoning effort label.
    /// - `auth_mode`: auth mode label (for example `subscription` or `api`).
    /// - `cwd`: current working directory text shown in the banner.
    /// - `git_branch`: optional git branch label.
    ///
    /// Output:
    /// - A `RenderMeta` value consumed by output rendering.
    pub const fn new(
        model: String,
        reasoning: String,
        auth_mode: String,
        cwd: String,
        git_branch: Option<String>,
    ) -> Self {
        Self {
            model,
            reasoning,
            auth_mode,
            cwd,
            git_branch,
        }
    }
}

/// Draws one full TUI frame.
///
/// Inputs:
/// - `frame`: current ratatui frame to render into.
/// - `state`: immutable UI state used for read-only rendering decisions.
/// - `meta`: static banner metadata for this session.
///
/// Behavior:
/// - Builds output content.
/// - Computes the vertical layout split (output, status, input).
/// - Draws output text, optional running status line, input box, and cursor.
///
/// Output:
/// - Rendered output snapshot used for clipboard selection extraction.
pub(super) fn draw(
    frame: &mut Frame<'_>,
    state: &TuiState,
    meta: &RenderMeta,
) -> OutputRenderSnapshot {
    let plan = build_render_plan(frame.area(), state, meta);
    let output_snapshot = draw_output(
        frame,
        &plan.output,
        plan.output_area,
        plan.output_scroll_top,
        state.output_selection_range(),
    );
    if state.status_row_visible() {
        status_pane::draw(frame, state, plan.status_area);
    }
    input_pane::draw_input(frame, state, plan.input_area);
    input_pane::place_input_cursor(frame, state, plan.input_area);

    output_snapshot
}

pub(super) fn layout_context(state: &TuiState, meta: &RenderMeta, area: Rect) -> LayoutContext {
    let plan = build_render_plan(area, state, meta);
    LayoutContext::new(
        plan.input_inner_width,
        plan.max_output_scroll_lines_from_bottom,
        OutputViewport {
            x: plan.output_area.x,
            y: plan.output_area.y,
            width: plan.output_area.width,
            height: plan.output_area.height,
        },
    )
}

fn build_render_plan(area: Rect, state: &TuiState, meta: &RenderMeta) -> RenderPlan {
    let output = build_output_view(state, meta);
    let show_status = state.status_row_visible();
    let status_height_rows = u16::from(show_status);
    let max_input_height_rows = area.height.saturating_sub(status_height_rows);
    let input_height_rows =
        input_pane::height_rows(state.input(), area.width, max_input_height_rows);
    let max_output_height_rows = area
        .height
        .saturating_sub(input_height_rows.saturating_add(status_height_rows));
    let output_height_rows =
        output::height_rows(&output.plain_lines, area.width, max_output_height_rows);

    let [output_area, status_area, input_area, _rest] = Layout::vertical([
        Constraint::Length(output_height_rows),
        Constraint::Length(status_height_rows),
        Constraint::Length(input_height_rows),
        Constraint::Min(0),
    ])
    .areas(area);

    let max_output_scroll_lines_from_bottom =
        viewport::max_scroll_offset(&output.plain_lines, output_area.width, output_area.height);
    let output_scroll_top = viewport::scroll_offset_from_max(
        max_output_scroll_lines_from_bottom,
        state.output_scroll_lines_from_bottom(),
    );
    let input_inner_width = input_area
        .inner(Margin {
            vertical: 1,
            horizontal: 0,
        })
        .width;

    RenderPlan {
        output,
        output_area,
        status_area,
        input_area,
        output_scroll_top,
        max_output_scroll_lines_from_bottom,
        input_inner_width,
    }
}
