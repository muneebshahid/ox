use super::truncate;

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

struct ReadArgs<'a> {
    path: &'a str,
    offset: usize,
    limit: Option<usize>,
}

fn parse_args(args: &serde_json::Value) -> Result<ReadArgs<'_>, String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| "Error: missing 'path' argument".to_string())?;

    let offset = args["offset"]
        .as_u64()
        .and_then(|o| usize::try_from(o.saturating_sub(1)).ok())
        .unwrap_or(0);

    let limit = args["limit"]
        .as_u64()
        .and_then(|l| usize::try_from(l).ok());

    Ok(ReadArgs {
        path,
        offset,
        limit,
    })
}

fn execute(args: &ReadArgs) -> Result<String, String> {
    let content =
        std::fs::read_to_string(args.path).map_err(|e| format!("Error: {e}"))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if args.offset >= total_lines {
        return Err(format!(
            "Error: offset {} is beyond end of file ({total_lines} lines)",
            args.offset
        ));
    }

    let end = args
        .limit
        .map_or(total_lines, |l| (args.offset + l).min(total_lines));

    let selected = lines[args.offset..end].join("\n");
    let output = truncate::head(&selected, 2000, &format!(
        "lines remaining, use offset={} to continue",
        args.offset + 2000 + 1 // next 1-indexed line after truncation
    ));

    Ok(output)
}

pub fn run(args: &serde_json::Value) -> String {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    match execute(&args) {
        Ok(output) => output,
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn read_small_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3").unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap() }));
        assert!(result.contains("line1"));
        assert!(result.contains("line3"));
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn read_with_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3").unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap(), "offset": 2 }));
        assert!(!result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
    }

    #[test]
    fn read_with_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3").unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap(), "limit": 1 }));
        assert!(result.contains("line1"));
        assert!(!result.contains("line2"));
    }

    #[test]
    fn offset_beyond_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2").unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap(), "offset": 100 }));
        assert!(result.contains("Error"));
        assert!(result.contains("beyond end"));
    }

    #[test]
    fn truncation_includes_next_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        // Create a file with more than 2000 lines
        let content: String = (1..=2500).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        fs::write(&path, &content).unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap() }));
        assert!(result.contains("truncated"));
        assert!(result.contains("offset=2001"));
    }

    #[test]
    fn missing_path() {
        let result = run(&json!({}));
        assert!(result.contains("Error"));
    }

    #[test]
    fn file_not_found() {
        let result = run(&json!({ "path": "/nonexistent/file.txt" }));
        assert!(result.contains("Error"));
    }
}
