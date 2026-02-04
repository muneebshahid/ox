pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "grep",
        "description": "Search for a text pattern in files within a directory. Returns matching lines with file paths and line numbers.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The text pattern to search for" },
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

    let result = std::process::Command::new("rg")
        .args(["-n", "--no-heading", pattern, path])
        .output()
        .or_else(|_| {
            std::process::Command::new("grep")
                .args(["-rn", pattern, path])
                .output()
        });

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                format!("No matches found for '{pattern}'")
            } else {
                stdout.into_owned()
            }
        }
        Err(e) => format!("Error: {e}"),
    }
}
