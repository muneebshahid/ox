const MAX_LINES: usize = 2000;
const MAX_BYTES: usize = 50 * 1024; // 50KB

/// Keep the first `max_lines` lines or `MAX_BYTES`, whichever hits first.
pub fn head(text: &str, max_lines: usize, label: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= max_lines && text.len() <= MAX_BYTES {
        return text.to_string();
    }

    let mut byte_count = 0;
    let mut line_count = 0;

    for line in &lines {
        let line_bytes = byte_count + line.len() + 1; // +1 for newline
        if line_count >= max_lines || line_bytes > MAX_BYTES {
            break;
        }
        byte_count = line_bytes;
        line_count += 1;
    }

    let truncated = &text[..byte_count];
    let remaining = lines.len() - line_count;
    format!("{truncated}\n... truncated ({remaining} {label})")
}

/// Keep the last N lines / bytes. Used for command output.
pub fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= MAX_LINES && text.len() <= MAX_BYTES {
        return text.to_string();
    }

    // Start from the end, collect lines until we hit limits
    let mut byte_count = 0;
    let mut line_count = 0;

    for line in lines.iter().rev() {
        let line_bytes = byte_count + line.len() + 1; // +1 for newline
        if line_count >= MAX_LINES || line_bytes > MAX_BYTES {
            break;
        }
        byte_count = line_bytes;
        line_count += 1;
    }

    let omitted = lines.len() - line_count;
    let kept = &lines[omitted..];

    format!(
        "... truncated ({omitted} lines omitted)\n{}",
        kept.join("\n")
    )
}
