#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct OutputViewport {
    pub(in crate::tui) x: u16,
    pub(in crate::tui) y: u16,
    pub(in crate::tui) width: u16,
    pub(in crate::tui) height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct CellPos {
    pub(in crate::tui) col: u16,
    pub(in crate::tui) row: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) struct OutputRenderSnapshot {
    pub(in crate::tui) viewport: OutputViewport,
    pub(in crate::tui) cells: Vec<Vec<String>>,
}

pub(in crate::tui) fn selected_text(
    snapshot: &OutputRenderSnapshot,
    start: CellPos,
    end: CellPos,
) -> Option<String> {
    let cells = &snapshot.cells;
    if cells.is_empty() {
        return None;
    }

    let height = cells.len();
    let width = cells.first().map_or(0, Vec::len);
    if width == 0 || height == 0 {
        return None;
    }

    let start_row = usize::from(start.row).min(height - 1);
    let end_row = usize::from(end.row).min(height - 1);
    let start_col = usize::from(start.col).min(width - 1);
    let end_col = usize::from(end.col).min(width - 1);
    let (first_row, first_col, last_row, last_col) = if (end_row, end_col) < (start_row, start_col)
    {
        (end_row, end_col, start_row, start_col)
    } else {
        (start_row, start_col, end_row, end_col)
    };

    let mut lines = Vec::new();
    for row in first_row..=last_row {
        let from_col = if row == first_row { first_col } else { 0 };
        let to_col = if row == last_row { last_col } else { width - 1 };
        let mut line = String::new();
        for col in from_col..=to_col {
            if let Some(cell) = cells.get(row).and_then(|cells_row| cells_row.get(col)) {
                line.push_str(cell);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    let text = lines.join("\n").trim_end().to_string();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::{CellPos, OutputRenderSnapshot, OutputViewport, selected_text};

    fn snapshot(rows: &[&str]) -> OutputRenderSnapshot {
        OutputRenderSnapshot {
            viewport: OutputViewport {
                x: 0,
                y: 0,
                width: rows.first().map_or(0, |row| row.chars().count() as u16),
                height: rows.len() as u16,
            },
            cells: rows
                .iter()
                .map(|row| row.chars().map(|ch| ch.to_string()).collect())
                .collect(),
        }
    }

    #[test]
    fn selected_text_extracts_multiline_range() {
        let snap = snapshot(&["ABCDEF", "GHIJKL", "MNOPQR"]);
        let text = selected_text(
            &snap,
            CellPos { col: 1, row: 0 },
            CellPos { col: 3, row: 1 },
        );
        assert_eq!(text, Some("BCDEF\nGHIJ".to_string()));
    }
}
