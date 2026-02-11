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

    #[serde(rename = "response.completed")]
    ResponseCompleted {
        response: Option<ResponseCompletedPayload>,
    },

    #[serde(rename = "response.done")]
    ResponseDone {
        response: Option<ResponseCompletedPayload>,
    },

    #[serde(rename = "response.failed")]
    ResponseFailed {
        response: Option<ResponseFailedPayload>,
    },

    #[serde(rename = "error")]
    Error {
        code: Option<String>,
        message: Option<String>,
    },

    #[serde(other)]
    Ignored,
}

#[derive(Deserialize, Debug)]
pub(super) struct ResponseCompletedPayload {
    pub(super) status: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct ResponseFailedPayload {
    pub(super) status: Option<String>,
    pub(super) error: Option<ResponseError>,
}

#[derive(Deserialize, Debug)]
pub(super) struct ResponseError {
    pub(super) code: Option<String>,
    pub(super) message: Option<String>,
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
