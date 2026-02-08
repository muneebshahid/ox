use super::truncate;
use std::process::Command;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "find",
        "description": "Find files by name pattern. Searches recursively in the given directory.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The filename pattern to match (e.g. '*.rs', 'main.*')" },
                "path": { "type": "string", "description": "The directory to search in (default: '.')" },
                "limit": { "type": "integer", "description": "Maximum number of results to return (default: 1000)" }
            },
            "required": ["pattern"]
        }
    })
}

struct FindArgs<'a> {
    pattern: &'a str,
    path: &'a str,
    limit: usize,
}

fn parse_args(args: &serde_json::Value) -> Result<FindArgs<'_>, String> {
    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| "Error: missing 'pattern' argument".to_string())?;

    Ok(FindArgs {
        pattern,
        path: args["path"].as_str().unwrap_or("."),
        limit: usize::try_from(args["limit"].as_u64().unwrap_or(1000)).unwrap_or(1000),
    })
}

fn execute(args: &FindArgs) -> Result<String, String> {
    let output = Command::new("fd")
        .args(["--glob", args.pattern, args.path])
        .output()
        .or_else(|_| {
            Command::new("find")
                .args([args.path, "-name", args.pattern])
                .output()
        })
        .map_err(|e| format!("Error: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn format_output(stdout: &str, pattern: &str, limit: usize) -> String {
    if stdout.is_empty() {
        format!("No files found matching '{pattern}'")
    } else {
        truncate::head(stdout, limit, "results remaining")
    }
}

pub fn run(args: &serde_json::Value) -> String {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let stdout = match execute(&args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    format_output(&stdout, args.pattern, args.limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(dir.path().join("notes.txt"), "hello").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/mod.rs"), "mod sub;").unwrap();
        dir
    }

    #[test]
    fn find_by_extension() {
        let dir = setup_test_dir();
        let result = run(&json!({ "pattern": "*.rs", "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("mod.rs"));
        assert!(!result.contains("notes.txt"));
    }

    #[test]
    fn find_specific_file() {
        let dir = setup_test_dir();
        let result = run(&json!({ "pattern": "notes.txt", "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("notes.txt"));
        assert!(!result.contains(".rs"));
    }

    #[test]
    fn no_matches() {
        let dir = setup_test_dir();
        let result = run(&json!({ "pattern": "*.py", "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("No files found"));
    }

    #[test]
    fn missing_pattern() {
        let result = run(&json!({}));
        assert!(result.contains("Error"));
    }

    #[test]
    fn limit_results() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap(),
            "limit": 1
        }));
        let file_lines: Vec<&str> = result.lines().filter(|l| l.contains(".rs")).collect();
        assert_eq!(file_lines.len(), 1);
    }
}
