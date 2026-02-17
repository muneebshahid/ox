use crate::tui::output_surface::OutputViewport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct LayoutContext {
    pub(super) input_inner_width: u16,
    pub(super) max_output_scroll_lines_from_bottom: u16,
    pub(super) output_viewport: OutputViewport,
}

impl LayoutContext {
    pub(in crate::tui) const fn new(
        input_inner_width: u16,
        max_output_scroll_lines_from_bottom: u16,
        output_viewport: OutputViewport,
    ) -> Self {
        Self {
            input_inner_width,
            max_output_scroll_lines_from_bottom,
            output_viewport,
        }
    }

    pub(in crate::tui) const fn empty() -> Self {
        Self {
            input_inner_width: 0,
            max_output_scroll_lines_from_bottom: 0,
            output_viewport: OutputViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        }
    }
}
