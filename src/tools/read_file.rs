pub fn definition() -> serde_json::Value {
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
    })
}

pub fn run(args: &serde_json::Value) -> String {
    let Some(path) = args["path"].as_str() else {
        return "Error: missing 'path' argument".to_string();
    };
    std::fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {e}"))
}
