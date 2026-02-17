use std::io::{self, Write};
use std::process::{Command, Stdio};

pub(super) fn copy_to_clipboard(text: &str) -> io::Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    copy_with_platform_command(text).or_else(|_| copy_osc52(text))
}

fn copy_with_platform_command(text: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return run_command_with_stdin("pbcopy", &[], text);
    }

    #[cfg(target_os = "windows")]
    {
        return run_command_with_stdin("clip", &[], text);
    }

    #[cfg(target_os = "linux")]
    {
        for (program, args) in [
            ("wl-copy", Vec::new()),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ] {
            if run_command_with_stdin(program, &args, text).is_ok() {
                return Ok(());
            }
        }
        return Err(io::Error::other(
            "no supported Linux clipboard command available",
        ));
    }

    #[allow(unreachable_code)]
    Err(io::Error::other(
        "clipboard command fallback unsupported on this platform",
    ))
}

fn run_command_with_stdin(program: &str, args: &[&str], input: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let write_result = (|| -> io::Result<()> {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("clipboard command stdin unavailable"))?;
        stdin.write_all(input.as_bytes())?;
        Ok(())
    })();

    let wait_result = child.wait();
    match (write_result, wait_result) {
        (Err(write_err), Err(wait_err)) => Err(io::Error::other(format!(
            "clipboard command {program} write failed: {write_err}; wait failed: {wait_err}"
        ))),
        (Err(write_err), Ok(_)) => Err(write_err),
        (Ok(()), Err(wait_err)) => Err(wait_err),
        (Ok(()), Ok(status)) if status.success() => Ok(()),
        (Ok(()), Ok(status)) => Err(io::Error::other(format!(
            "clipboard command {program} failed with {status}"
        ))),
    }
}

fn copy_osc52(text: &str) -> io::Result<()> {
    let encoded = base64_encode(text.as_bytes());
    let osc52 = format!("\x1b]52;c;{encoded}\x07");
    let mut stdout = io::stdout();
    stdout.write_all(osc52.as_bytes())?;
    stdout.flush()
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut index = 0;

    while index < input.len() {
        let chunk0 = u32::from(input[index]);
        let chunk1 = if index + 1 < input.len() {
            u32::from(input[index + 1])
        } else {
            0
        };
        let chunk2 = if index + 2 < input.len() {
            u32::from(input[index + 2])
        } else {
            0
        };

        let block = (chunk0 << 16) | (chunk1 << 8) | chunk2;
        output.push(TABLE[((block >> 18) & 0x3F) as usize] as char);
        output.push(TABLE[((block >> 12) & 0x3F) as usize] as char);
        if index + 1 < input.len() {
            output.push(TABLE[((block >> 6) & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
        if index + 2 < input.len() {
            output.push(TABLE[(block & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }

        index += 3;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::base64_encode;

    #[test]
    fn encodes_base64_without_padding_errors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }
}
