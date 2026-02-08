use super::truncate;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "ls",
        "description": "List files and directories at the given path.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The directory path to list (default: '.')" },
                "limit": { "type": "integer", "description": "Maximum number of entries to return (default: 500)" }
            }
        }
    })
}

struct LsArgs<'a> {
    path: &'a str,
    limit: usize,
}

fn parse_args(args: &serde_json::Value) -> LsArgs<'_> {
    LsArgs {
        path: args["path"].as_str().unwrap_or("."),
        limit: args["limit"]
            .as_u64()
            .and_then(|l| usize::try_from(l).ok())
            .unwrap_or(500),
    }
}

fn execute(args: &LsArgs) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(args.path).map_err(|e| format!("Error: {e}"))?;

    let mut items: Vec<String> = entries
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let suffix = if entry.path().is_dir() { "/" } else { "" };
            format!("{name}{suffix}")
        })
        .collect();
    items.sort();
    Ok(items)
}

fn format_output(items: &[String], limit: usize) -> String {
    truncate::head(&items.join("\n"), limit, "entries remaining")
}

pub fn run(args: &serde_json::Value) -> String {
    let args = parse_args(args);
    match execute(&args) {
        Ok(items) => format_output(&items, args.limit),
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn list_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        let result = run(&json!({ "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));
        assert!(result.contains("subdir/"));
    }

    #[test]
    fn defaults_to_current_dir() {
        let result = run(&json!({}));
        assert!(!result.contains("Error"));
    }

    #[test]
    fn invalid_path() {
        let result = run(&json!({ "path": "/nonexistent/dir" }));
        assert!(result.contains("Error"));
    }

    #[test]
    fn limit_results() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            fs::write(dir.path().join(format!("file{i}.txt")), "").unwrap();
        }
        let result = run(&json!({ "path": dir.path().to_str().unwrap(), "limit": 3 }));
        let file_lines: Vec<&str> = result.lines().filter(|l| l.contains("file")).collect();
        assert_eq!(file_lines.len(), 3);
    }

    #[test]
    fn sorted_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();
        let result = run(&json!({ "path": dir.path().to_str().unwrap() }));
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines, vec!["a.txt", "b.txt", "c.txt"]);
    }
}
