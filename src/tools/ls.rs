use super::truncate;
use super::ToolResult;

pub fn definition() -> serde_json::Value {
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
    })
}

pub fn run(args: &serde_json::Value) -> ToolResult {
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
            ToolResult::success(truncate::head(&items.join("\n"), 500, "entries remaining"))
        }
        Err(e) => ToolResult::error(format!("Error: {e}")),
    }
}
