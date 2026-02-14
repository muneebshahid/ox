/// Returns output pane height clamped to both content size and available space.
///
/// Inputs:
/// - `lines`: plain output lines.
/// - `width`: viewport width in cells used for wrapping.
/// - `max_output_height`: maximum rows available after layout reservations.
///
/// Behavior:
/// - Computes wrapped visual line count for `lines`.
/// - Caps that count at `max_output_height`.
///
/// Example:
/// - `lines = ["abcdefgh", "ij"]`, `width = 4`, `max_output_height = 2`
/// - Wrapped line count is `3` (`2 + 1`), so result is `min(3, 2) = 2`.
///
/// Output:
/// - Number of rows to allocate to output.
pub(super) fn clamped_output_height(lines: &[String], width: u16, max_output_height: u16) -> u16 {
    if max_output_height == 0 {
        return 0;
    }

    count_wrapped_lines(lines, width).min(max_output_height)
}

/// Computes the top-of-buffer scroll offset for the output paragraph.
///
/// Inputs:
/// - `lines`: plain output lines.
/// - `width`: viewport width in cells used for wrapping.
/// - `viewport_height`: output viewport height in rows.
/// - `scroll_lines_from_bottom`: manual scroll distance measured from bottom.
///
/// Behavior:
/// - Computes maximum valid top offset for current content/viewport.
/// - Converts bottom-relative manual scroll into top offset.
///
/// Example:
/// - `lines = ["abcdefgh", "ij"]`, `width = 4`, `viewport_height = 2`
/// - Total wrapped lines is `3`, so max top offset is `1`.
/// - If `scroll_lines_from_bottom = 0`, `scroll_offset = 1` (follow bottom).
/// - If `scroll_lines_from_bottom = 1`, `scroll_offset = 0` (scroll one line up).
///
/// Output:
/// - Scroll offset (`y`) passed to `Paragraph::scroll`.
pub(super) fn scroll_offset(
    lines: &[String],
    width: u16,
    viewport_height: u16,
    scroll_lines_from_bottom: u16,
) -> u16 {
    let max_offset = max_scroll_offset(lines, width, viewport_height);
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
    use super::{clamped_output_height, max_scroll_offset, scroll_offset, wrapped_line_count};

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
    fn clamped_output_height_uses_total_wrapped_lines_when_it_fits() {
        let source = lines(&["abc", "def"]);
        assert_eq!(clamped_output_height(&source, 10, 10), 2);
    }

    #[test]
    fn clamped_output_height_clamps_to_available_space() {
        let source = lines(&["abcdefgh", "ijklmnop"]);
        assert_eq!(clamped_output_height(&source, 4, 3), 3);
    }

    #[test]
    fn clamped_output_height_handles_zero_max_height() {
        let source = lines(&["abc"]);
        assert_eq!(clamped_output_height(&source, 10, 0), 0);
    }

    #[test]
    fn scroll_offset_is_zero_when_content_fits_viewport() {
        let source = lines(&["abc", "def"]);
        assert_eq!(scroll_offset(&source, 10, 5, 0), 0);
    }

    #[test]
    fn scroll_offset_moves_to_bottom_when_content_overflows() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(scroll_offset(&source, 2, 3, 0), 3);
    }

    #[test]
    fn scroll_offset_changes_when_width_changes() {
        let source = lines(&["abcdefghij"]);
        let narrow = scroll_offset(&source, 3, 2, 0);
        let wide = scroll_offset(&source, 10, 2, 0);
        assert_eq!(narrow, 2);
        assert_eq!(wide, 0);
    }

    #[test]
    fn scroll_offset_returns_zero_for_zero_height_viewport() {
        let source = lines(&["abcdef"]);
        assert_eq!(scroll_offset(&source, 3, 0, 0), 0);
    }

    #[test]
    fn scroll_offset_saturates_for_large_input() {
        let very_long_line = "x".repeat(usize::from(u16::MAX) * 2);
        let source = vec![very_long_line];
        assert_eq!(scroll_offset(&source, 1, 1, 0), u16::MAX - 1);
    }

    #[test]
    fn scroll_offset_moves_up_when_manual_scroll_is_set() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(scroll_offset(&source, 2, 3, 2), 1);
    }

    #[test]
    fn scroll_offset_clamps_to_top_when_manual_scroll_is_larger_than_content() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(scroll_offset(&source, 2, 3, 99), 0);
    }

    #[test]
    fn max_scroll_offset_matches_bottom_offset_when_following_output() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(max_scroll_offset(&source, 2, 3), 3);
    }
}
