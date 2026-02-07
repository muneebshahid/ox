use super::truncate;
use super::ToolResult;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "read_file",
        "description": "Read the contents of a file. Output is truncated to 2000 lines or 50KB. Use offset/limit for large files.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The file path to read" },
                "offset": { "type": "integer", "description": "Line number to start reading from (1-indexed)" },
                "limit": { "type": "integer", "description": "Maximum number of lines to read" }
            },
            "required": ["path"]
        }
    })
}

pub fn run(args: &serde_json::Value) -> ToolResult {
    let Some(path) = args["path"].as_str() else {
        return ToolResult::error("Error: missing 'path' argument");
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::error(format!("Error: {e}")),
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Apply offset (1-indexed)
    let start = args["offset"]
        .as_u64()
        .and_then(|o| usize::try_from(o.saturating_sub(1)).ok())
        .unwrap_or(0);

    if start >= total_lines {
        return ToolResult::error(format!(
            "Error: offset {start} is beyond end of file ({total_lines} lines)"
        ));
    }

    // Apply limit
    let end = args["limit"]
        .as_u64()
        .and_then(|l| usize::try_from(l).ok())
        .map_or(total_lines, |l| (start + l).min(total_lines));

    let selected = lines[start..end].join("\n");
    ToolResult::success(truncate::head(
        &selected,
        2000,
        "lines remaining, use offset to read more",
    ))
}
