pub const MAX_LINES: usize = 2000;
pub const MAX_BYTES: usize = 50 * 1024; // 50KB

/// Count how many items from `iter` fit within `max_lines` and `MAX_BYTES`.
/// Each item's byte size is its length plus one (for the newline).
/// Returns `(lines_counted, total_bytes)`.
fn count_within_limits<'a>(
    iter: impl Iterator<Item = &'a str>,
    max_lines: usize,
) -> (usize, usize) {
    let mut byte_count = 0;
    let mut line_count = 0;

    for line in iter {
        let next_bytes = byte_count + line.len() + 1;
        if line_count >= max_lines || next_bytes > MAX_BYTES {
            break;
        }
        byte_count = next_bytes;
        line_count += 1;
    }

    (line_count, byte_count)
}

/// Keep the first `max_lines` lines or `MAX_BYTES`, whichever hits first.
pub fn head(text: &str, max_lines: usize, label: &str) -> String {
    let (kept, bytes) = count_within_limits(text.lines(), max_lines);
    let total = text.lines().count();

    if kept == total {
        return text.to_string();
    }

    let truncated = &text[..bytes];
    let remaining = total - kept;
    format!("{truncated}\n... truncated ({remaining} {label})")
}

/// Keep the last `MAX_LINES` lines or `MAX_BYTES`, whichever hits first.
pub fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    let (kept, _) = count_within_limits(lines.iter().rev().copied(), MAX_LINES);

    if kept == lines.len() {
        return text.to_string();
    }

    let omitted = lines.len() - kept;
    let start = lines[omitted].as_ptr() as usize - text.as_ptr() as usize;
    let tail_text = &text[start..];

    format!("... truncated ({omitted} lines omitted)\n{tail_text}")
}
