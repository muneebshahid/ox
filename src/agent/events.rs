use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(super) enum StreamEvent {
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: serde_json::Value },

    #[serde(rename = "response.output_text.delta")]
    TextDelta { delta: String },

    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: serde_json::Value },

    #[serde(other)]
    Ignored,
}

#[derive(Deserialize)]
pub(super) struct FunctionCallItem {
    #[serde(default)]
    pub(super) call_id: String,
    #[serde(default)]
    pub(super) name: String,
    #[serde(default)]
    pub(super) arguments: serde_json::Value,
}

pub(super) fn parse_function_call_item(item: &serde_json::Value) -> Option<FunctionCallItem> {
    if item.get("type").and_then(serde_json::Value::as_str) != Some("function_call") {
        return None;
    }
    serde_json::from_value(item.clone()).ok()
}
