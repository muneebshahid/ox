use super::truncate;
use std::process::Command;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "grep",
        "description": "Search for a text pattern in files within a directory. Returns matching lines with file paths and line numbers.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The text pattern to search for (regex by default)" },
                "path": { "type": "string", "description": "The directory or file to search in (default: '.')" },
                "glob": { "type": "string", "description": "Filter files by glob pattern (e.g. '*.rs', '*.{ts,tsx}')" },
                "ignore_case": { "type": "boolean", "description": "Case-insensitive search (default: false)" },
                "literal": { "type": "boolean", "description": "Treat pattern as a literal string, not regex (default: false)" },
                "context": { "type": "integer", "description": "Number of lines to show before and after each match (default: 0)" },
                "limit": { "type": "integer", "description": "Maximum number of matching lines to return (default: 100)" }
            },
            "required": ["pattern"]
        }
    })
}

struct GrepArgs<'a> {
    pattern: &'a str,
    path: &'a str,
    glob: Option<&'a str>,
    ignore_case: bool,
    literal: bool,
    context: Option<u64>,
    limit: usize,
}

fn parse_args(args: &serde_json::Value) -> Result<GrepArgs<'_>, String> {
    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| "Error: missing 'pattern' argument".to_string())?;

    Ok(GrepArgs {
        pattern,
        path: args["path"].as_str().unwrap_or("."),
        glob: args["glob"].as_str(),
        ignore_case: args["ignore_case"].as_bool().unwrap_or(false),
        literal: args["literal"].as_bool().unwrap_or(false),
        context: args["context"].as_u64(),
        limit: usize::try_from(args["limit"].as_u64().unwrap_or(100)).unwrap_or(100),
    })
}

fn execute(args: &GrepArgs) -> Result<String, String> {
    let mut cmd = Command::new("rg");
    cmd.args(["-n", "--no-heading"]);

    if args.ignore_case {
        cmd.arg("-i");
    }
    if args.literal {
        cmd.arg("-F");
    }
    if let Some(glob) = args.glob {
        cmd.args(["--glob", glob]);
    }
    if let Some(ctx) = args.context {
        cmd.args(["-C", &ctx.to_string()]);
    }

    cmd.args([args.pattern, args.path]);

    let output = cmd
        .output()
        .or_else(|_| {
            Command::new("grep")
                .args(["-rn", args.pattern, args.path])
                .output()
        })
        .map_err(|e| format!("Error: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn format_output(stdout: &str, pattern: &str, limit: usize) -> String {
    if stdout.is_empty() {
        format!("No matches found for '{pattern}'")
    } else {
        truncate::head(stdout, limit, "matches remaining")
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
        fs::write(
            dir.path().join("hello.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("lib.rs"),
            "pub fn greet() {\n    println!(\"hello world\");\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("notes.txt"),
            "Hello there\nhello again\nGoodbye\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn basic_search() {
        let dir = setup_test_dir();
        let result = run(&json!({ "pattern": "Hello", "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("Hello"));
        assert!(!result.contains("No matches"));
    }

    #[test]
    fn no_matches() {
        let dir = setup_test_dir();
        let result = run(&json!({ "pattern": "ZZZZZ", "path": dir.path().to_str().unwrap() }));
        assert!(result.contains("No matches found"));
    }

    #[test]
    fn missing_pattern() {
        let result = run(&json!({}));
        assert!(result.contains("Error"));
    }

    #[test]
    fn glob_filter() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap(),
            "glob": "*.txt",
            "ignore_case": true
        }));
        assert!(result.contains("notes.txt"));
        assert!(!result.contains(".rs"));
    }

    #[test]
    fn ignore_case() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap(),
            "ignore_case": true
        }));
        // Should match both "Hello" and "hello"
        assert!(result.contains("Hello"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn case_sensitive_by_default() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap()
        }));
        assert!(result.contains("hello"));
        assert!(!result.contains("Hello"));
    }

    #[test]
    fn literal_mode() {
        let dir = setup_test_dir();
        // "." as regex would match everything; as literal it should match nothing in our files
        let result = run(&json!({
            "pattern": "fn.main",
            "path": dir.path().to_str().unwrap(),
            "literal": true
        }));
        assert!(result.contains("No matches"));
    }

    #[test]
    fn regex_mode_by_default() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "fn.main",
            "path": dir.path().to_str().unwrap()
        }));
        assert!(result.contains("fn main"));
    }

    #[test]
    fn context_lines() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "hello again",
            "path": dir.path().to_str().unwrap(),
            "context": 1
        }));
        // Context should include surrounding lines
        assert!(result.contains("Hello there"));
        assert!(result.contains("Goodbye"));
    }

    #[test]
    fn limit() {
        let dir = setup_test_dir();
        let result = run(&json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap(),
            "ignore_case": true,
            "limit": 1
        }));
        // Should only have 1 matching line
        let match_lines: Vec<&str> = result
            .lines()
            .filter(|l| l.contains("hello") || l.contains("Hello"))
            .collect();
        assert_eq!(match_lines.len(), 1);
    }
}
