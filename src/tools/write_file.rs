use super::ToolResult;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "write_file",
        "description": "Write content to a file, creating it if it doesn't exist or overwriting if it does",
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

pub fn run(args: &serde_json::Value) -> ToolResult {
    let Some(path) = args["path"].as_str() else {
        return ToolResult::error("Error: missing 'path' argument");
    };
    let Some(content) = args["content"].as_str() else {
        return ToolResult::error("Error: missing 'content' argument");
    };
    match std::fs::write(path, content) {
        Ok(()) => ToolResult::success(format!("Successfully wrote to {path}")),
        Err(e) => ToolResult::error(format!("Error: {e}")),
    }
}
