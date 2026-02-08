use super::truncate;
use std::fmt::Write;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const MAX_LINES: usize = 2000;
const MAX_BYTES: usize = 50 * 1024;

pub fn definition() -> serde_json::Value {
    let description = format!(
        "Execute a shell command and return its output. Output is truncated to the last {} lines or {}KB. If truncated full output is saved to a temp file. Optionally provide timeout in seconds.",
        MAX_LINES,
        MAX_BYTES / 1024
    );
    serde_json::json!({
        "type": "function",
        "name": "bash",
        "description": description,
        "parameters": {
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" },
                "timeout": { "type": "integer", "description": "Timeout in seconds (optional, no default timeout)" }
            },
            "required": ["command"]
        }
    })
}

struct BashArgs<'a> {
    command: &'a str,
    timeout: Option<Duration>,
}

fn parse_args(args: &serde_json::Value) -> Result<BashArgs<'_>, String> {
    let command = args["command"]
        .as_str()
        .ok_or_else(|| "Error: missing 'command' argument".to_string())?;

    let timeout = args["timeout"]
        .as_u64()
        .filter(|&s| s > 0)
        .map(Duration::from_secs);

    Ok(BashArgs { command, timeout })
}

struct CommandOutcome {
    status: ExitStatus,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

fn execute(args: &BashArgs) -> Result<CommandOutcome, String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(args.command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error: {e}"))?;

    let stdout_thread = child.stdout.take().map(spawn_reader);
    let stderr_thread = child.stderr.take().map(spawn_reader);

    let mut timed_out = false;
    let status = wait_for_exit(&mut child, args.timeout, &mut timed_out)?;
    let stdout_bytes = join_reader(stdout_thread);
    let stderr_bytes = join_reader(stderr_thread);

    Ok(CommandOutcome {
        status,
        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
        timed_out,
    })
}

fn format_output(outcome: &CommandOutcome, timeout: Option<Duration>) -> String {
    let exit_code = outcome.status.code().unwrap_or(-1);
    let result = combine_streams(&outcome.stdout, &outcome.stderr);

    if outcome.timed_out {
        return format_timeout_error(timeout, &result);
    }

    if !outcome.status.success() {
        if result.is_empty() {
            return format!("Error: command exited with code {exit_code}");
        }
        return format!(
            "Error: command exited with code {exit_code}\n{}",
            tail_with_tempfile(&result)
        );
    }

    if result.is_empty() {
        format!("Command exited with code {exit_code}")
    } else {
        tail_with_tempfile(&result)
    }
}

pub fn run(args: &serde_json::Value) -> String {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let outcome = match execute(&args) {
        Ok(o) => o,
        Err(e) => return e,
    };

    format_output(&outcome, args.timeout)
}

fn wait_for_exit(
    child: &mut std::process::Child,
    timeout: Option<Duration>,
    timed_out: &mut bool,
) -> Result<ExitStatus, String> {
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if let Some(limit) = timeout
                    && start.elapsed() >= limit
                {
                    *timed_out = true;
                    let _ = child.kill();
                    return child
                        .wait()
                        .map_err(|e| format!("Error: failed waiting for process: {e}"));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(format!("Error: failed to poll process status: {e}")),
        }
    }
}

fn spawn_reader<R>(mut pipe: R) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        buf
    })
}

fn join_reader(handle: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default()
}

fn combine_streams(stdout: &str, stderr: &str) -> String {
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        let _ = write!(result, "stderr: {stderr}");
    }
    result
}

/// If output exceeds limits, save full output to a temp file and append the
/// path to the truncated result so the LLM can `read_file` it.
fn tail_with_tempfile(text: &str) -> String {
    if text.lines().count() <= MAX_LINES && text.len() <= MAX_BYTES {
        return text.to_string();
    }

    let truncated = truncate::tail(text);
    match save_to_tempfile(text) {
        Ok(path) => format!("{truncated}\n\n(full output saved to {path})"),
        Err(e) => format!("{truncated}\n\n(failed to save full output: {e})"),
    }
}

fn save_to_tempfile(content: &str) -> Result<String, std::io::Error> {
    let dir = std::env::temp_dir();
    let name = format!("ox-bash-{:x}.log", std::process::id());
    let path = dir.join(name);
    std::fs::write(&path, content)?;
    Ok(path.to_string_lossy().into_owned())
}

fn format_timeout_error(timeout: Option<Duration>, result: &str) -> String {
    if result.is_empty() {
        return timeout.map_or_else(
            || "Error: command timed out".to_string(),
            |limit| format!("Error: command timed out after {} seconds", limit.as_secs()),
        );
    }

    timeout.map_or_else(
        || format!("Error: command timed out\n{}", tail_with_tempfile(result)),
        |limit| {
            format!(
                "Error: command timed out after {} seconds\n{}",
                limit.as_secs(),
                tail_with_tempfile(result)
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_command() {
        let result = run(&json!({ "command": "echo hello" }));
        assert_eq!(result.trim(), "hello");
    }

    #[test]
    fn missing_command() {
        let result = run(&json!({}));
        assert!(result.contains("Error"));
        assert!(result.contains("missing"));
    }

    #[test]
    fn exit_code_on_failure() {
        let result = run(&json!({ "command": "exit 42" }));
        assert!(result.contains("Error"));
        assert!(result.contains("42"));
    }

    #[test]
    fn stderr_included() {
        let result = run(&json!({ "command": "echo oops >&2" }));
        assert!(result.contains("stderr:"));
        assert!(result.contains("oops"));
    }

    #[test]
    fn stdout_and_stderr_combined() {
        let result = run(&json!({ "command": "echo out && echo err >&2" }));
        assert!(result.contains("out"));
        assert!(result.contains("stderr: err"));
    }

    #[test]
    fn empty_output_shows_exit_code() {
        let result = run(&json!({ "command": "true" }));
        assert!(result.contains("Command exited with code 0"));
    }

    #[test]
    fn timeout_kills_command() {
        let result = run(&json!({ "command": "sleep 60", "timeout": 1 }));
        assert!(result.contains("timed out"));
        assert!(result.contains("1 seconds"));
    }

    #[test]
    fn invalid_timeout_ignored() {
        let result = run(&json!({ "command": "echo hi", "timeout": -1 }));
        assert_eq!(result.trim(), "hi");
    }

    #[test]
    fn zero_timeout_ignored() {
        let result = run(&json!({ "command": "echo hi", "timeout": 0 }));
        assert_eq!(result.trim(), "hi");
    }

    #[test]
    fn null_timeout_ignored() {
        let result = run(&json!({ "command": "echo hi", "timeout": null }));
        assert_eq!(result.trim(), "hi");
    }

    #[test]
    fn large_output_truncated_with_tempfile() {
        // Generate output exceeding MAX_LINES
        let cmd = format!("seq 1 {}", MAX_LINES + 500);
        let result = run(&json!({ "command": cmd }));
        assert!(result.contains("full output saved to"));
    }

    #[test]
    fn combine_streams_both_empty() {
        assert_eq!(combine_streams("", ""), "");
    }

    #[test]
    fn combine_streams_stdout_only() {
        assert_eq!(combine_streams("hello", ""), "hello");
    }

    #[test]
    fn combine_streams_stderr_only() {
        assert_eq!(combine_streams("", "fail"), "stderr: fail");
    }

    #[test]
    fn combine_streams_both() {
        let result = combine_streams("out", "err");
        assert_eq!(result, "out\nstderr: err");
    }
}
