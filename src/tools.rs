use anyhow::{Context, Result};

pub fn definitions() -> Vec<serde_json::Value> {
    vec![serde_json::json!({
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
    })]
}

pub fn execute(name: &str, arguments: &str) -> Result<String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments).context("failed to parse tool arguments")?;

    match name {
        "read_file" => {
            let path = args["path"]
                .as_str()
                .context("missing 'path' argument for read_file")?;
            Ok(std::fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {e}")))
        }
        _ => Ok(format!("Unknown tool: {name}")),
    }
}
