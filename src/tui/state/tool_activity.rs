use std::fmt::Write as _;

pub(super) fn format_tool_start(tool_name: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<serde_json::Value>(args_json).ok();

    if let Some(args) = args.as_ref()
        && let Some(message) = format_known_tool(tool_name, args)
    {
        return message;
    }

    let summary = args.as_ref().map_or_else(
        || truncate_preview(args_json, 120),
        |value| truncate_preview(&value.to_string(), 120),
    );
    if summary.is_empty() {
        format!("[tool] running {tool_name}")
    } else {
        format!("[tool] running {tool_name} {summary}")
    }
}

fn format_known_tool(tool_name: &str, args: &serde_json::Value) -> Option<String> {
    match tool_name {
        "read_file" => {
            let path = arg_str(args, "path")?;
            let mut message = format!("[tool] reading {}", truncate_preview(path, 100));
            match (arg_u64(args, "offset"), arg_u64(args, "limit")) {
                (Some(offset), Some(limit)) if limit > 0 => {
                    let end = offset.saturating_add(limit).saturating_sub(1);
                    let _ = write!(message, ":{offset}-{end}");
                }
                (Some(offset), None) => {
                    let _ = write!(message, ":{offset}-");
                }
                (None, Some(limit)) => {
                    let _ = write!(message, " (limit {limit})");
                }
                _ => {}
            }
            Some(message)
        }
        "ls" => {
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!("[tool] listing {}", truncate_preview(path, 100)))
        }
        "bash" => {
            let command = arg_str(args, "command")?.replace('\n', " ");
            Some(format!("[tool] bash: {}", truncate_preview(&command, 120)))
        }
        "write_file" => {
            let path = arg_str(args, "path")?;
            Some(format!("[tool] writing {}", truncate_preview(path, 100)))
        }
        "edit" => {
            let path = arg_str(args, "path")?;
            Some(format!("[tool] editing {}", truncate_preview(path, 100)))
        }
        "grep" => {
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!(
                "[tool] grep /{}/ in {}",
                truncate_preview(pattern, 60),
                truncate_preview(path, 80)
            ))
        }
        "find" => {
            let pattern = arg_str(args, "pattern").unwrap_or("");
            let path = arg_str(args, "path").unwrap_or(".");
            Some(format!(
                "[tool] finding {} in {}",
                truncate_preview(pattern, 60),
                truncate_preview(path, 80)
            ))
        }
        _ => None,
    }
}

fn arg_str<'a>(args: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(serde_json::Value::as_str)
}

fn arg_u64(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(serde_json::Value::as_u64)
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max_chars).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::{format_tool_start, truncate_preview};

    #[test]
    fn formats_known_tools_with_read_file_range_variants() {
        let with_offset_only =
            format_tool_start("read_file", r#"{"path":"src/main.rs","offset":42}"#);
        assert_eq!(with_offset_only, "[tool] reading src/main.rs:42-");

        let with_limit_only = format_tool_start("read_file", r#"{"path":"src/main.rs","limit":3}"#);
        assert_eq!(with_limit_only, "[tool] reading src/main.rs (limit 3)");
    }

    #[test]
    fn formats_bash_commands_with_newlines_as_single_line() {
        let formatted = format_tool_start("bash", r#"{"command":"echo hello\necho world"}"#);
        assert_eq!(formatted, "[tool] bash: echo hello echo world");
    }

    #[test]
    fn falls_back_when_tool_args_are_not_json() {
        let formatted = format_tool_start("mystery_tool", "raw-args");
        assert_eq!(formatted, "[tool] running mystery_tool raw-args");
    }

    #[test]
    fn falls_back_to_tool_name_only_when_args_are_empty() {
        let formatted = format_tool_start("mystery_tool", "");
        assert_eq!(formatted, "[tool] running mystery_tool");
    }

    #[test]
    fn truncate_preview_appends_ellipsis_when_exceeding_limit() {
        assert_eq!(truncate_preview("hello", 5), "hello");
        assert_eq!(truncate_preview("hello world", 5), "hello...");
    }
}
