/// Returns pane height from wrapped content plus fixed non-content rows.
///
/// Inputs:
/// - `lines`: plain content lines.
/// - `width`: viewport width in cells used for wrapping.
/// - `max_height`: maximum rows available for the pane.
/// - `fixed_rows`: rows reserved inside the pane (for example borders).
///
/// Behavior:
/// - Computes wrapped content height for `lines`.
/// - Clamps content to `max_height - fixed_rows`.
/// - Adds `fixed_rows` back and clamps final height to `max_height`.
///
/// Output:
/// - Total pane height in rows.
pub(super) fn row_height(lines: &[String], width: u16, max_height: u16, fixed_rows: u16) -> u16 {
    if max_height == 0 {
        return 0;
    }

    let max_content_height = max_height.saturating_sub(fixed_rows);
    let content_height = count_wrapped_lines(lines, width).min(max_content_height);
    content_height.saturating_add(fixed_rows).min(max_height)
}

/// Converts a bottom-relative manual scroll into a top-of-buffer offset.
///
/// Inputs:
/// - `max_offset`: maximum valid top offset for current content/viewport.
/// - `scroll_lines_from_bottom`: manual scroll distance measured from bottom.
///
/// Output:
/// - Top-of-buffer offset (`y`) passed to `Paragraph::scroll`.
pub(super) const fn scroll_offset_from_max(max_offset: u16, scroll_lines_from_bottom: u16) -> u16 {
    max_offset.saturating_sub(scroll_lines_from_bottom)
}

/// Computes the largest top-of-buffer offset that still shows content.
///
/// Inputs:
/// - `lines`: plain output lines.
/// - `viewport_width`: viewport width in cells used for wrapping.
/// - `viewport_height`: viewport height in rows.
///
/// Example:
/// - `lines = ["abcdefgh", "ij"]`, `viewport_width = 4`, `viewport_height = 2`
/// - Wrapped line counts are `2` and `1`, so total wrapped lines is `3`.
/// - `max_scroll_offset = 3 - 2 = 1`.
///
/// Output:
/// - Maximum valid top offset; `0` means content fits without scrolling.
pub(super) fn max_scroll_offset(
    lines: &[String],
    viewport_width: u16,
    viewport_height: u16,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }

    count_wrapped_lines(lines, viewport_width).saturating_sub(viewport_height)
}

/// Counts total visual (wrapped) rows for a list of logical lines.
///
/// Inputs:
/// - `lines`: plain lines to measure.
/// - `width`: wrapping width in cells.
///
/// Behavior:
/// - Sums wrapped row count for each line.
/// - Uses saturating arithmetic and guarantees at least 1 visual row.
///
/// Examples:
/// - `lines = ["abcde", "xy"]`, `width = 3`
/// - Wrapped counts are `2` and `1`, so `count_wrapped_lines = 3`.
///
/// - `lines = ["abcdef", "jklmnop"]`, `width = 2`
/// - Wrapped counts are `3` and `4`, so `count_wrapped_lines = 7`.
///
/// Output:
/// - Total wrapped row count as `u16`.
fn count_wrapped_lines(lines: &[String], width: u16) -> u16 {
    if width == 0 {
        return 1;
    }

    let width = usize::from(width);
    lines
        .iter()
        .map(|line| wrapped_line_count(line, width))
        .fold(0_u16, u16::saturating_add)
        .max(1)
}

/// Counts wrapped visual rows for a single logical line.
///
/// Inputs:
/// - `line`: logical line content.
/// - `width`: wrapping width in cells (must be non-zero).
///
/// Behavior:
/// - Treats empty lines as one visual row.
/// - Uses ceil division to compute wraps.
/// - Saturates to `u16::MAX` on very large values.
///
/// Example:
/// - `wrapped_line_count("abcdef", 4) = 2`
/// - `wrapped_line_count("", 4) = 1`
///
/// Output:
/// - Visual row count for `line`.
fn wrapped_line_count(line: &str, width: usize) -> u16 {
    let chars = line.chars().count().max(1);
    u16::try_from(chars.div_ceil(width)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::{max_scroll_offset, row_height, scroll_offset_from_max, wrapped_line_count};

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|line| (*line).to_string()).collect()
    }

    #[test]
    fn wrapped_line_count_counts_single_line_when_shorter_than_width() {
        assert_eq!(wrapped_line_count("abc", 10), 1);
    }

    #[test]
    fn wrapped_line_count_wraps_when_longer_than_width() {
        assert_eq!(wrapped_line_count("abcdefgh", 3), 3);
    }

    #[test]
    fn wrapped_line_count_treats_empty_line_as_one_visual_line() {
        assert_eq!(wrapped_line_count("", 5), 1);
    }

    #[test]
    fn row_height_adds_fixed_rows_after_clamping_content() {
        let source = lines(&["abcdef"]);
        assert_eq!(row_height(&source, 3, 4, 2), 4);
    }

    #[test]
    fn row_height_respects_zero_max_height() {
        let source = lines(&["abcdef"]);
        assert_eq!(row_height(&source, 3, 0, 2), 0);
    }

    #[test]
    fn scroll_offset_from_max_is_zero_when_content_fits_viewport() {
        let source = lines(&["abc", "def"]);
        let max = max_scroll_offset(&source, 10, 5);
        assert_eq!(scroll_offset_from_max(max, 0), 0);
    }

    #[test]
    fn scroll_offset_from_max_moves_to_bottom_when_content_overflows() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        let max = max_scroll_offset(&source, 2, 3);
        assert_eq!(scroll_offset_from_max(max, 0), 3);
    }

    #[test]
    fn scroll_offset_from_max_changes_when_width_changes() {
        let source = lines(&["abcdefghij"]);
        let narrow = scroll_offset_from_max(max_scroll_offset(&source, 3, 2), 0);
        let wide = scroll_offset_from_max(max_scroll_offset(&source, 10, 2), 0);
        assert_eq!(narrow, 2);
        assert_eq!(wide, 0);
    }

    #[test]
    fn scroll_offset_from_max_returns_zero_for_zero_height_viewport() {
        let source = lines(&["abcdef"]);
        let max = max_scroll_offset(&source, 3, 0);
        assert_eq!(scroll_offset_from_max(max, 0), 0);
    }

    #[test]
    fn scroll_offset_from_max_saturates_for_large_input() {
        let very_long_line = "x".repeat(usize::from(u16::MAX) * 2);
        let source = vec![very_long_line];
        let max = max_scroll_offset(&source, 1, 1);
        assert_eq!(scroll_offset_from_max(max, 0), u16::MAX - 1);
    }

    #[test]
    fn scroll_offset_from_max_moves_up_when_manual_scroll_is_set() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        let max = max_scroll_offset(&source, 2, 3);
        assert_eq!(scroll_offset_from_max(max, 2), 1);
    }

    #[test]
    fn scroll_offset_from_max_clamps_to_top_when_manual_scroll_is_larger_than_content() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        let max = max_scroll_offset(&source, 2, 3);
        assert_eq!(scroll_offset_from_max(max, 99), 0);
    }

    #[test]
    fn max_scroll_offset_matches_bottom_offset_when_following_output() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(max_scroll_offset(&source, 2, 3), 3);
    }

    #[test]
    fn scroll_offset_from_max_clamps_when_bottom_scroll_exceeds_max() {
        assert_eq!(scroll_offset_from_max(3, 99), 0);
    }
}
