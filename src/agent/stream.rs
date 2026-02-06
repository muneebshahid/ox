use super::events::StreamEvent;

pub(super) fn parse_event(data: &str) -> Option<StreamEvent> {
    match serde_json::from_str(data) {
        Ok(event) => Some(event),
        Err(err) => {
            let preview: String = data.chars().take(200).collect();
            eprintln!("Warning: failed to parse SSE event JSON: {err}; payload: {preview}");
            None
        }
    }
}

pub(super) fn get_event(buffer: &mut String) -> Option<String> {
    loop {
        let pos = buffer.find("\n\n")?;
        let event_text = buffer[..pos].to_string();
        *buffer = buffer[pos + 2..].to_string();

        let data_lines: Vec<String> = event_text
            .lines()
            .filter_map(|line| {
                let data = line.strip_prefix("data:")?;
                Some(data.strip_prefix(' ').unwrap_or(data).to_string())
            })
            .collect();

        if !data_lines.is_empty() {
            return Some(data_lines.join("\n"));
        }
    }
}
