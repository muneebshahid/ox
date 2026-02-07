use super::ToolResult;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "edit",
        "description": "Edit a file by replacing exact text. The old_text must match exactly including whitespace. Use read_file first to see the current content.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The file path to edit" },
                "old_text": { "type": "string", "description": "The exact text to find (must match exactly, must be unique in the file)" },
                "new_text": { "type": "string", "description": "The text to replace it with" }
            },
            "required": ["path", "old_text", "new_text"]
        }
    })
}

pub fn run(args: &serde_json::Value) -> ToolResult {
    let Some(path) = args["path"].as_str() else {
        return ToolResult::error("Error: missing 'path' argument");
    };
    let Some(old_text) = args["old_text"].as_str() else {
        return ToolResult::error("Error: missing 'old_text' argument");
    };
    let Some(new_text) = args["new_text"].as_str() else {
        return ToolResult::error("Error: missing 'new_text' argument");
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::error(format!("Error reading file: {e}")),
    };

    let count = content.matches(old_text).count();
    match count {
        0 => ToolResult::error("Error: old_text not found in file"),
        1 => {
            let new_content = content.replacen(old_text, new_text, 1);
            match std::fs::write(path, new_content) {
                Ok(()) => ToolResult::success(format!("Successfully edited {path}")),
                Err(e) => ToolResult::error(format!("Error writing file: {e}")),
            }
        }
        n => ToolResult::error(format!(
            "Error: old_text found {n} times, include more surrounding context to make it unique"
        )),
    }
}
