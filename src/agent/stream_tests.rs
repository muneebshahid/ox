use super::events::StreamEvent;
use super::stream::{get_event, parse_event};

#[test]
fn get_event_returns_first_data_payload() {
    let mut buffer = "event: response\ndata: {\"type\":\"x\"}\n\n".to_string();

    let data = get_event(&mut buffer);

    assert_eq!(data, Some("{\"type\":\"x\"}".to_string()));
    assert!(buffer.is_empty());
}

#[test]
fn get_event_skips_non_data_events() {
    let mut buffer = "event: ping\n\n\
                      event: response\ndata: {\"type\":\"y\"}\n\n"
        .to_string();

    let data = get_event(&mut buffer);

    assert_eq!(data, Some("{\"type\":\"y\"}".to_string()));
    assert!(buffer.is_empty());
}

#[test]
fn get_event_joins_multi_line_data() {
    let mut buffer = "event: response\n\
                      data: {\"type\":\"response.output_text.delta\",\n\
                      data: \"delta\":\"Hello\"}\n\n"
        .to_string();

    let data = get_event(&mut buffer);

    assert_eq!(
        data,
        Some("{\"type\":\"response.output_text.delta\",\n\"delta\":\"Hello\"}".to_string())
    );
    assert!(buffer.is_empty());
}

#[test]
fn parse_event_rejects_invalid_json() {
    let result = parse_event("{not-json");
    assert!(result.is_none());
}

#[test]
fn parse_event_accepts_known_event() {
    let data = r#"{"type":"response.output_text.delta","delta":"hi"}"#;
    let event = parse_event(data).expect("event should parse");
    assert!(matches!(event, StreamEvent::TextDelta { .. }));
}
