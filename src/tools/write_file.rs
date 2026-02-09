use std::path::Path;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "write_file",
        "description": "Write content to a file, creating it and any parent directories if they don't exist, or overwriting if it does.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The file path to write to" },
                "content": { "type": "string", "description": "The content to write to the file" }
            },
            "required": ["path", "content"]
        }
    })
}

struct WriteArgs<'a> {
    path: &'a str,
    content: &'a str,
}

fn parse_args(args: &serde_json::Value) -> Result<WriteArgs<'_>, String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| "Error: missing 'path' argument".to_string())?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| "Error: missing 'content' argument".to_string())?;
    Ok(WriteArgs { path, content })
}

fn execute(args: &WriteArgs) -> Result<String, String> {
    if let Some(parent) = Path::new(args.path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Error creating directories: {e}"))?;
    }
    std::fs::write(args.path, args.content).map_err(|e| format!("Error: {e}"))?;
    Ok(format!("Successfully wrote to {}", args.path))
}

pub fn run(args: &serde_json::Value) -> String {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };

    match execute(&args) {
        Ok(msg) => msg,
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let result = run(&json!({ "path": path.to_str().unwrap(), "content": "hello" }));
        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "old").unwrap();
        let result = run(&json!({ "path": path.to_str().unwrap(), "content": "new" }));
        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.txt");
        let result = run(&json!({ "path": path.to_str().unwrap(), "content": "deep" }));
        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "deep");
    }

    #[test]
    fn missing_path() {
        let result = run(&json!({ "content": "hello" }));
        assert!(result.contains("Error"));
    }

    #[test]
    fn missing_content() {
        let result = run(&json!({ "path": "/tmp/test.txt" }));
        assert!(result.contains("Error"));
    }
}
