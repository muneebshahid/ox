use super::truncate;

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "bash",
        "description": "Execute a shell command and return its output. Output is truncated to the last 2000 lines or 50KB.",
        "parameters": {
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" }
            },
            "required": ["command"]
        }
    })
}

pub fn run(args: &serde_json::Value) -> String {
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
                format!(
                    "Command exited with code {}",
                    output.status.code().unwrap_or(-1)
                )
            } else {
                truncate::tail(&result)
            }
        }
        Err(e) => format!("Error: {e}"),
    }
}
