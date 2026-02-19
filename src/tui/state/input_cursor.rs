use crate::tui::input_metrics;

#[derive(Clone, Copy)]
pub(super) enum VerticalDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VerticalMoveTarget {
    pub(super) byte_offset: usize,
    pub(super) preferred_column: u16,
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
    preferred_column: Option<u16>,
) -> Option<VerticalMoveTarget> {
    if width == 0 {
        return None;
    }

    let positions = cursor_positions_with_prompt(input, width);
    let current = positions
        .iter()
        .find(|pos| pos.byte == current_cursor_byte_offset)
        .copied()?;
    let current_row_start = row_start_col(&positions, current.row)?;
    let preferred_column =
        preferred_column.unwrap_or_else(|| current.col.saturating_sub(current_row_start));

    let target_row = match direction {
        VerticalDirection::Up => {
            if current.row == 0 {
                return None;
            }
            current.row - 1
        }
        VerticalDirection::Down => current.row.saturating_add(1),
    };

    let target_row_start = row_start_col(&positions, target_row)?;
    let target = closest_position_on_row(
        &positions,
        target_row,
        target_row_start.saturating_add(preferred_column),
    )?;
    (target.byte != current_cursor_byte_offset).then_some(VerticalMoveTarget {
        byte_offset: target.byte,
        preferred_column,
    })
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

fn row_start_col(positions: &[CursorPosition], row: u16) -> Option<u16> {
    positions
        .iter()
        .filter_map(|position| (position.row == row).then_some(position.col))
        .min()
}

#[cfg(test)]
mod tests {
    use super::{VerticalDirection, target_cursor_byte_offset_for_vertical_move};

    #[test]
    fn returns_cursor_positions_for_all_char_boundaries() {
        let target =
            target_cursor_byte_offset_for_vertical_move("ab", 2, 10, VerticalDirection::Up, None);
        assert_eq!(target, None);
    }

    #[test]
    fn moves_up_and_down_across_wrapped_rows() {
        let input = "abcdefghij";
        let end = input.len();

        let up =
            target_cursor_byte_offset_for_vertical_move(input, end, 5, VerticalDirection::Up, None)
                .expect("expected up target");
        assert!(up.byte_offset < end);

        let down = target_cursor_byte_offset_for_vertical_move(
            input,
            up.byte_offset,
            5,
            VerticalDirection::Down,
            Some(up.preferred_column),
        )
        .expect("expected down target");
        assert_eq!(down.byte_offset, end);
    }

    #[test]
    fn keeps_trailing_newline_on_next_row() {
        let input = "hello\n";
        let current = input.len();

        let up = target_cursor_byte_offset_for_vertical_move(
            input,
            current,
            20,
            VerticalDirection::Up,
            None,
        )
        .expect("expected up target");
        assert!(up.byte_offset < current);
    }

    #[test]
    fn moving_up_to_first_input_row_ignores_prompt_offset() {
        let input = "12345\n789";
        let current = "12345\n789".len();

        let up = target_cursor_byte_offset_for_vertical_move(
            input,
            current,
            40,
            VerticalDirection::Up,
            None,
        )
        .expect("expected up target");

        assert_eq!(up.byte_offset, 3);
    }

    #[test]
    fn repeated_vertical_moves_keep_original_preferred_column() {
        let input = "12345\n789\nabcdfe";
        let current = input.len();

        let first_up = target_cursor_byte_offset_for_vertical_move(
            input,
            current,
            40,
            VerticalDirection::Up,
            None,
        )
        .expect("expected first up target");
        assert_eq!(first_up.byte_offset, 9);

        let second_up = target_cursor_byte_offset_for_vertical_move(
            input,
            first_up.byte_offset,
            40,
            VerticalDirection::Up,
            Some(first_up.preferred_column),
        )
        .expect("expected second up target");
        assert_eq!(second_up.byte_offset, 5);
    }
}
