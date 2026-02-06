use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(super) enum StreamEvent {
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: OutputItem },

    #[serde(rename = "response.output_text.delta")]
    TextDelta { delta: String },

    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: OutputItem },

    #[serde(other)]
    Ignored,
}

#[derive(Deserialize, Debug)]
pub(super) struct OutputItem {
    #[serde(rename = "type")]
    pub(super) item_type: String,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) call_id: Option<String>,
    #[serde(default)]
    pub(super) arguments: Option<String>,
    #[serde(default)]
    pub(super) content: Vec<OutputContentPart>,
}

#[derive(Deserialize, Debug)]
pub(super) struct OutputContentPart {
    #[serde(default)]
    pub(super) text: Option<String>,
}
