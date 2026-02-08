use std::path::{Path, PathBuf};

const CONTEXT_FILENAMES: &[&str] = &["AGENTS.md", "CLAUDE.md"];

fn discover_context_files(cwd: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut current = Some(cwd);

    while let Some(dir) = current {
        for filename in CONTEXT_FILENAMES {
            let candidate = dir.join(filename);
            if candidate.exists() {
                files.push(candidate);
            }
        }
        current = dir.parent();
    }

    files.reverse();
    files
}

fn build_context_files(cwd: &Path) -> String {
    let mut sections = Vec::new();
    for path in discover_context_files(cwd) {
        match std::fs::read_to_string(&path) {
            Ok(content) => sections.push(format!("## {}\n\n{content}", path.display())),
            Err(error) => eprintln!(
                "Warning: failed to read context from {}: {error}",
                path.display()
            ),
        }
    }

    if sections.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nProject-specific instructions and guidelines:\n\n{}",
            sections.join("\n\n")
        )
    }
}

fn current_datetime() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_datetime() {
        println!("{}", current_datetime());
    }
}

pub fn build() -> String {
    let cwd_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd = cwd_path.to_string_lossy().to_string();
    let context = build_context_files(&cwd_path);
    let datetime = current_datetime();

    format!(
        "\
You are an expert coding assistant operating inside ox, a terminal-based coding agent. \
You help users by reading files, executing commands, editing code, and writing new files.

Available tools:
- read_file: Read file contents
- ls: List directory contents
- write_file: Create or overwrite files
- edit: Make surgical edits to files (find exact text and replace)
- grep: Search file contents for patterns
- find: Find files by name pattern
- bash: Execute shell commands

Guidelines:
- Use read_file to examine files before editing
- Use edit for precise changes (old_text must match exactly)
- Use write_file only for new files or complete rewrites
- Prefer grep and find over bash for file exploration
- Be concise in your responses
- Show file paths clearly when working with files

Current date and time: {datetime}
Working directory: {cwd}{context}"
    )
}
