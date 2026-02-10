mod normalize;

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

struct EditArgs<'a> {
    path: &'a str,
    old_text: &'a str,
    new_text: &'a str,
}

fn parse_args(args: &serde_json::Value) -> Result<EditArgs<'_>, String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| "Error: missing 'path' argument".to_string())?;
    let old_text = args["old_text"]
        .as_str()
        .ok_or_else(|| "Error: missing 'old_text' argument".to_string())?;
    let new_text = args["new_text"]
        .as_str()
        .ok_or_else(|| "Error: missing 'new_text' argument".to_string())?;
    Ok(EditArgs {
        path,
        old_text,
        new_text,
    })
}

fn execute(args: &EditArgs) -> Result<String, String> {
    let raw_content =
        std::fs::read_to_string(args.path).map_err(|e| format!("Error reading file: {e}"))?;

    let (bom, content) = normalize::strip_bom(&raw_content);
    let crlf = normalize::is_crlf(content);
    let mut base = content.replace("\r\n", "\n");
    let mut old_text = args.old_text.replace("\r\n", "\n");
    let new_text = args.new_text.replace("\r\n", "\n");

    let mut occurrences = base.matches(&*old_text).count();
    if occurrences == 0 {
        base = normalize::replace_special_chars(&base);
        old_text = normalize::replace_special_chars(&old_text);
        occurrences = base.matches(&*old_text).count();
    }

    if old_text.is_empty() || occurrences == 0 {
        return Err("Error: old_text not found in file".to_string());
    }
    if occurrences > 1 {
        return Err(format!(
            "Error: old_text found {occurrences} times, include more surrounding context to make it unique"
        ));
    }

    let replaced = base.replacen(&*old_text, &new_text, 1);
    if base == replaced {
        return Err(format!(
            "Error: no changes made to {}, old_text and new_text produce identical content",
            args.path
        ));
    }

    let final_content = format!("{bom}{}", normalize::restore_line_endings(&replaced, crlf));
    std::fs::write(args.path, final_content).map_err(|e| format!("Error writing file: {e}"))?;

    Ok(format!("Successfully edited {}", args.path))
}

pub fn run(args: &serde_json::Value) -> String {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    match execute(&args) {
        Ok(msg) => msg,
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn basic_edit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "hello",
            "new_text": "goodbye"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "goodbye world");
    }

    #[test]
    fn not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "missing",
            "new_text": "new"
        }));
        assert!(result.contains("not found"));
    }

    #[test]
    fn multiple_occurrences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "aaa").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "a",
            "new_text": "b"
        }));
        assert!(result.contains("3 times"));
    }

    #[test]
    fn preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\nline3").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "line2",
            "new_text": "changed"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "line1\r\nchanged\r\nline3"
        );
    }

    #[test]
    fn preserves_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "\u{FEFF}hello world").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "hello",
            "new_text": "goodbye"
        }));
        assert!(result.contains("Successfully edited"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with('\u{FEFF}'));
        assert!(content.contains("goodbye world"));
    }

    #[test]
    fn missing_args() {
        assert!(run(&json!({})).contains("Error"));
        assert!(run(&json!({"path": "x"})).contains("Error"));
        assert!(run(&json!({"path": "x", "old_text": "y"})).contains("Error"));
    }

    #[test]
    fn no_actual_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "hello",
            "new_text": "hello"
        }));
        assert!(result.contains("no changes"));
    }

    #[test]
    fn mixed_special_chars() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, r#"println!("a-b")"#).unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "println!(\u{201C}a\u{2013}b\u{201D})",
            "new_text": "println!(\"x\")"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(fs::read_to_string(&path).unwrap(), r#"println!("x")"#);
    }

    #[test]
    fn non_breaking_space_in_indentation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "fn main() {\n    return 1;\n}").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "\u{00A0}\u{00A0}\u{00A0}\u{00A0}return 1;",
            "new_text": "    return 2;"
        }));
        assert!(result.contains("Successfully edited"));
        assert!(fs::read_to_string(&path).unwrap().contains("return 2;"));
    }

    #[test]
    fn special_chars_in_new_text_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "hello",
            "new_text": "good\u{2014}bye"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "good\u{2014}bye");
    }

    #[test]
    fn file_has_smart_quotes_llm_sends_ascii() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "say \u{201C}hello\u{201D}").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "say \"hello\"",
            "new_text": "say \"world\""
        }));
        assert!(result.contains("Successfully edited"));
    }

    #[test]
    fn consecutive_special_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "a  b").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "a\u{00A0}\u{2003}b",
            "new_text": "a b"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "a b");
    }

    #[test]
    fn crlf_in_old_text_matches_lf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2").unwrap();
        let result = run(&json!({
            "path": path.to_str().unwrap(),
            "old_text": "line1\r\nline2",
            "new_text": "changed"
        }));
        assert!(result.contains("Successfully edited"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed");
    }
}
