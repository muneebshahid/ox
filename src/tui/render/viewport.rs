pub(super) fn clamped_output_height(lines: &[String], width: u16, max_output_height: u16) -> u16 {
    if max_output_height == 0 {
        return 0;
    }

    count_wrapped_lines(lines, width).min(max_output_height)
}

pub(super) fn scroll_offset(lines: &[String], width: u16, viewport_height: u16) -> u16 {
    if viewport_height == 0 {
        return 0;
    }

    let total_lines = count_wrapped_lines(lines, width);
    total_lines.saturating_sub(viewport_height)
}

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

fn wrapped_line_count(line: &str, width: usize) -> u16 {
    let chars = line.chars().count().max(1);
    u16::try_from(chars.div_ceil(width)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::{clamped_output_height, scroll_offset, wrapped_line_count};

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
        assert_eq!(scroll_offset(&source, 10, 5), 0);
    }

    #[test]
    fn scroll_offset_moves_to_bottom_when_content_overflows() {
        let source = lines(&["abcd", "efgh", "ijkl"]);
        assert_eq!(scroll_offset(&source, 2, 3), 3);
    }

    #[test]
    fn scroll_offset_changes_when_width_changes() {
        let source = lines(&["abcdefghij"]);
        let narrow = scroll_offset(&source, 3, 2);
        let wide = scroll_offset(&source, 10, 2);
        assert_eq!(narrow, 2);
        assert_eq!(wide, 0);
    }

    #[test]
    fn scroll_offset_returns_zero_for_zero_height_viewport() {
        let source = lines(&["abcdef"]);
        assert_eq!(scroll_offset(&source, 3, 0), 0);
    }

    #[test]
    fn scroll_offset_saturates_for_large_input() {
        let very_long_line = "x".repeat(usize::from(u16::MAX) * 2);
        let source = vec![very_long_line];
        assert_eq!(scroll_offset(&source, 1, 1), u16::MAX - 1);
    }
}
