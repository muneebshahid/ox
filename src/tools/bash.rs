use super::truncate;
use super::ToolResult;
use std::fmt::Write;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub fn definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": "bash",
        "description": "Execute a shell command and return its output. Output is truncated to the last 2000 lines or 50KB. Optionally provide timeout in seconds.",
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

pub fn run(args: &serde_json::Value) -> ToolResult {
    let Some(command) = args["command"].as_str() else {
        return ToolResult::error("Error: missing 'command' argument");
    };

    let timeout = match parse_timeout(args) {
        Ok(timeout) => timeout,
        Err(err) => return ToolResult::error(err),
    };

    let outcome = match execute_command(command, timeout) {
        Ok(outcome) => outcome,
        Err(err) => return ToolResult::error(err),
    };

    let exit_code = outcome.status.code().unwrap_or(-1);
    let result = combine_streams(&outcome.stdout, &outcome.stderr);

    if outcome.timed_out {
        return ToolResult::error(format_timeout_error(timeout, &result));
    }

    if !outcome.status.success() {
        if result.is_empty() {
            return ToolResult::error(format!("Error: command exited with code {exit_code}"));
        }
        return ToolResult::error(format!(
            "Error: command exited with code {exit_code}\n{}",
            truncate::tail(&result)
        ));
    }

    if result.is_empty() {
        ToolResult::success(format!("Command exited with code {exit_code}"))
    } else {
        ToolResult::success(truncate::tail(&result))
    }
}

fn parse_timeout(args: &serde_json::Value) -> Result<Option<Duration>, String> {
    match args.get("timeout") {
        Some(value) if !value.is_null() => match value.as_u64() {
            Some(seconds) if seconds > 0 => Ok(Some(Duration::from_secs(seconds))),
            _ => Err("Error: 'timeout' must be a positive integer".to_string()),
        },
        _ => Ok(None),
    }
}

struct CommandOutcome {
    status: ExitStatus,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

fn execute_command(command: &str, timeout: Option<Duration>) -> Result<CommandOutcome, String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error: {e}"))?;

    let stdout_thread = child.stdout.take().map(spawn_reader);
    let stderr_thread = child.stderr.take().map(spawn_reader);

    let mut timed_out = false;
    let status = wait_for_exit(&mut child, timeout, &mut timed_out)?;
    let stdout_bytes = join_reader(stdout_thread);
    let stderr_bytes = join_reader(stderr_thread);

    Ok(CommandOutcome {
        status,
        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
        timed_out,
    })
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

fn format_timeout_error(timeout: Option<Duration>, result: &str) -> String {
    if result.is_empty() {
        return timeout.map_or_else(
            || "Error: command timed out".to_string(),
            |limit| format!("Error: command timed out after {} seconds", limit.as_secs()),
        );
    }

    timeout.map_or_else(
        || format!("Error: command timed out\n{}", truncate::tail(result)),
        |limit| {
            format!(
                "Error: command timed out after {} seconds\n{}",
                limit.as_secs(),
                truncate::tail(result)
            )
        },
    )
}
