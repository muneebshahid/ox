use anyhow::{Context, Result};

pub fn definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "name": "read_file",
            "description": "Read the contents of a file at the given path",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "type": "function",
            "name": "ls",
            "description": "List files and directories at the given path",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to list. Defaults to current directory if not provided." }
                }
            }
        }),
    ]
}

pub fn execute(name: &str, arguments: &str) -> Result<String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments).context("failed to parse tool arguments")?;

    match name {
        "read_file" => read_file(&args),
        "ls" => ls(&args),
        _ => Ok(format!("Unknown tool: {name}")),
    }
}

fn read_file(args: &serde_json::Value) -> Result<String> {
    let path = args["path"]
        .as_str()
        .context("missing 'path' argument for read_file")?;
    Ok(std::fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {e}")))
}

fn ls(args: &serde_json::Value) -> Result<String> {
    let path = args["path"].as_str().unwrap_or(".");
    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut items: Vec<String> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let suffix = if entry.path().is_dir() { "/" } else { "" };
                items.push(format!("{name}{suffix}"));
            }
            items.sort();
            Ok(items.join("\n"))
        }
        Err(e) => Ok(format!("Error: {e}")),
    }
}
