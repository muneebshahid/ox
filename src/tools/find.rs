pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "find",
        "description": "Find files by name pattern. Searches recursively in the given directory.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The filename pattern to match (e.g. '*.rs', 'main.*')" },
                "path": { "type": "string", "description": "The directory to search in. Defaults to current directory." }
            },
            "required": ["pattern"]
        }
    })
}

pub fn run(args: &serde_json::Value) -> String {
    let Some(pattern) = args["pattern"].as_str() else {
        return "Error: missing 'pattern' argument".to_string();
    };
    let path = args["path"].as_str().unwrap_or(".");

    let result = std::process::Command::new("fd")
        .args(["--glob", pattern, path])
        .output()
        .or_else(|_| {
            std::process::Command::new("find")
                .args([path, "-name", pattern])
                .output()
        });

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                format!("No files found matching '{pattern}'")
            } else {
                stdout.into_owned()
            }
        }
        Err(e) => format!("Error: {e}"),
    }
}
