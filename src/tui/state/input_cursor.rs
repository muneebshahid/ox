use crate::tui::input_metrics;

#[derive(Clone, Copy)]
pub(super) enum VerticalDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CursorPosition {
    byte: usize,
    col: u16,
    row: u16,
}

pub(super) fn target_cursor_byte_offset_for_vertical_move(
    input: &str,
    current_cursor_byte_offset: usize,
    width: u16,
    direction: VerticalDirection,
) -> Option<usize> {
    if width == 0 {
        return None;
    }

    let positions = cursor_positions_with_prompt(input, width);
    let current = positions
        .iter()
        .find(|pos| pos.byte == current_cursor_byte_offset)
        .copied()?;

    let target_row = match direction {
        VerticalDirection::Up => {
            if current.row == 0 {
                return None;
            }
            current.row - 1
        }
        VerticalDirection::Down => current.row.saturating_add(1),
    };

    let target = closest_position_on_row(&positions, target_row, current.col)?;
    (target.byte != current_cursor_byte_offset).then_some(target.byte)
}

fn cursor_positions_with_prompt(input: &str, width: u16) -> Vec<CursorPosition> {
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

fn closest_position_on_row(
    positions: &[CursorPosition],
    row: u16,
    preferred_col: u16,
) -> Option<CursorPosition> {
    let mut best_lte: Option<CursorPosition> = None;
    let mut best_gt: Option<CursorPosition> = None;

    for &position in positions {
        if position.row != row {
            continue;
        }
        if position.col <= preferred_col {
            best_lte = match best_lte {
                Some(existing) if existing.col >= position.col => Some(existing),
                _ => Some(position),
            };
        } else {
            best_gt = match best_gt {
                Some(existing) if existing.col <= position.col => Some(existing),
                _ => Some(position),
            };
        }
    }

    best_lte.or(best_gt)
}

#[cfg(test)]
mod tests {
    use super::{VerticalDirection, target_cursor_byte_offset_for_vertical_move};

    #[test]
    fn returns_cursor_positions_for_all_char_boundaries() {
        let target =
            target_cursor_byte_offset_for_vertical_move("ab", 2, 10, VerticalDirection::Up);
        assert_eq!(target, None);
    }

    #[test]
    fn moves_up_and_down_across_wrapped_rows() {
        let input = "abcdefghij";
        let end = input.len();

        let up = target_cursor_byte_offset_for_vertical_move(input, end, 5, VerticalDirection::Up)
            .expect("expected up target");
        assert!(up < end);

        let down =
            target_cursor_byte_offset_for_vertical_move(input, up, 5, VerticalDirection::Down)
                .expect("expected down target");
        assert_eq!(down, end);
    }

    #[test]
    fn keeps_trailing_newline_on_next_row() {
        let input = "hello\n";
        let current = input.len();

        let up =
            target_cursor_byte_offset_for_vertical_move(input, current, 20, VerticalDirection::Up)
                .expect("expected up target");
        assert!(up < current);
    }
}
