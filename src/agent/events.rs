use serde::Deserialize;
use serde::de;
use serde::de::DeserializeOwned;

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

#[derive(Debug)]
pub(super) struct WithRaw<T> {
    pub(super) raw: serde_json::Value,
    pub(super) parsed: T,
}

impl<'de, T> Deserialize<'de> for WithRaw<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let parsed = serde_json::from_value(raw.clone()).map_err(de::Error::custom)?;

        Ok(Self { raw, parsed })
    }
}

pub(super) type OutputItem = WithRaw<OutputItemKind>;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(super) enum OutputItemKind {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        content: Vec<OutputContentPart>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        name: Option<String>,
        call_id: Option<String>,
        arguments: Option<String>,
    },
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
pub(super) struct OutputContentPart {
    pub(super) text: Option<String>,
}
