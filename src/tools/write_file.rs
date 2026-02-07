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

pub fn run(args: &serde_json::Value) -> String {
    let Some(path) = args["path"].as_str() else {
        return "Error: missing 'path' argument".to_string();
    };
    let Some(content) = args["content"].as_str() else {
        return "Error: missing 'content' argument".to_string();
    };
    match std::fs::write(path, content) {
        Ok(()) => format!("Successfully wrote to {path}"),
        Err(e) => format!("Error: {e}"),
    }
}
