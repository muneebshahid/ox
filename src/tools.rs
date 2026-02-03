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
        }),
        serde_json::json!({
            "type": "function",
            "name": "bash",
            "description": "Execute a shell command and return its output",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to execute" }
                },
                "required": ["command"]
            }
        }),
    ]
}

pub fn execute(name: &str, arguments: &str) -> String {
    let args: serde_json::Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(e) => return format!("Error parsing arguments: {e}"),
    };

    match name {
        "read_file" => read_file(&args),
        "ls" => ls(&args),
        "write_file" => write_file(&args),
        "bash" => bash(&args),
        _ => format!("Unknown tool: {name}"),
    }
}

fn read_file(args: &serde_json::Value) -> String {
    let Some(path) = args["path"].as_str() else {
        return "Error: missing 'path' argument".to_string();
    };
    std::fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {e}"))
}

fn write_file(args: &serde_json::Value) -> String {
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

fn bash(args: &serde_json::Value) -> String {
    use std::fmt::Write;
    let Some(command) = args["command"].as_str() else {
        return "Error: missing 'command' argument".to_string();
    };
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                let _ = write!(result, "stderr: {stderr}");
            }
            if result.is_empty() {
                format!("Command exited with code {}", output.status.code().unwrap_or(-1))
            } else {
                result
            }
        }
        Err(e) => format!("Error: {e}"),
    }
}

fn ls(args: &serde_json::Value) -> String {
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
            items.join("\n")
        }
        Err(e) => format!("Error: {e}"),
    }
}
