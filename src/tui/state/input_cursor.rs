use crate::tui::input_metrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorPosition {
    pub(super) byte: usize,
    pub(super) col: u16,
    pub(super) row: u16,
}

pub(super) fn cursor_positions_with_prompt(input: &str, width: u16) -> Vec<CursorPosition> {
    if width == 0 {
        return vec![CursorPosition {
            byte: 0,
            col: 0,
            row: 0,
        }];
    }

    let height = measurement_height_with_prompt(input, width);

    cursor_boundaries(input)
        .into_iter()
        .map(|byte| {
            let (col, row) = cursor_offset_with_prompt(&input[..byte], width, height);
            CursorPosition { byte, col, row }
        })
        .collect()
}

fn measurement_height_with_prompt(input: &str, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }

    let prompt_chars = input_metrics::INPUT_PROMPT_PREFIX.chars().count();
    let chars = input.chars().count();
    let newlines = input.chars().filter(|ch| *ch == '\n').count();
    let spaces = input
        .chars()
        .filter(|ch| ch.is_whitespace() && *ch != '\n')
        .count();

    let wrapped_chars = prompt_chars
        .saturating_add(chars)
        .div_ceil(usize::from(width));
    let upper_bound = wrapped_chars
        .saturating_add(newlines)
        .saturating_add(spaces)
        .saturating_add(2)
        .max(1);

    u16::try_from(upper_bound).unwrap_or(u16::MAX)
}

fn cursor_boundaries(input: &str) -> Vec<usize> {
    let mut boundaries: Vec<usize> = input.char_indices().map(|(idx, _)| idx).collect();
    if boundaries.first().is_none_or(|idx| *idx != 0) {
        boundaries.insert(0, 0);
    }
    boundaries.push(input.len());
    boundaries.dedup();
    boundaries
}

fn cursor_offset_with_prompt(input: &str, width: u16, height: u16) -> (u16, u16) {
    input_metrics::cursor_offset(
        &format!("{}{input}", input_metrics::INPUT_PROMPT_PREFIX),
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::cursor_positions_with_prompt;

    #[test]
    fn returns_cursor_positions_for_all_char_boundaries() {
        let positions = cursor_positions_with_prompt("ab", 10);
        let bytes: Vec<usize> = positions.iter().map(|pos| pos.byte).collect();
        assert_eq!(bytes, vec![0, 1, 2]);
    }

    #[test]
    fn keeps_trailing_newline_on_next_row() {
        let positions = cursor_positions_with_prompt("hello\n", 20);
        let last = positions.last().expect("position");
        assert_eq!(last.col, 0);
        assert_eq!(last.row, 1);
    }
}
