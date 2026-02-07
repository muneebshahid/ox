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

pub fn run(args: &serde_json::Value) -> String {
    let Some(path) = args["path"].as_str() else {
        return "Error: missing 'path' argument".to_string();
    };
    let Some(old_text) = args["old_text"].as_str() else {
        return "Error: missing 'old_text' argument".to_string();
    };
    let Some(new_text) = args["new_text"].as_str() else {
        return "Error: missing 'new_text' argument".to_string();
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return format!("Error reading file: {e}"),
    };

    let count = content.matches(old_text).count();
    match count {
        0 => "Error: old_text not found in file".to_string(),
        1 => {
            let new_content = content.replacen(old_text, new_text, 1);
            match std::fs::write(path, new_content) {
                Ok(()) => format!("Successfully edited {path}"),
                Err(e) => format!("Error writing file: {e}"),
            }
        }
        n => format!(
            "Error: old_text found {n} times, include more surrounding context to make it unique"
        ),
    }
}
