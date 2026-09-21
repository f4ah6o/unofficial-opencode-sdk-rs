use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A session returned by the legacy/current `/session` API.
///
/// Stable, commonly used fields are typed. Additional fields are retained so
/// new upstream fields do not make an older SDK fail to deserialize.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    #[serde(rename = "projectID")]
    pub project_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "workspaceID")]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    #[serde(rename = "parentID")]
    pub parent_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub time: Option<SessionTime>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
    #[serde(default)]
    pub compacting: Option<u64>,
    #[serde(default)]
    pub archived: Option<u64>,
}

/// Body accepted by `POST /session` at the pinned OpenCode snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none", rename = "parentID")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Kept as JSON for the first vertical slice because this nested contract
    /// is one of the schemas whose OpenAPI 3.1 unions are not handled correctly
    /// by the evaluated whole-document Rust generators.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PromptPart {
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        synthetic: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ignored: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
    },
    /// Escape hatch for current non-text prompt parts (file/agent/subtask).
    /// The outer request remains typed while this vertical slice avoids
    /// pretending unsupported OpenAPI unions are exhaustively generated.
    #[serde(untagged)]
    Other(Value),
}

impl PromptPart {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            id: None,
            text: text.into(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        }
    }
}

/// Body accepted by `POST /session/{id}/message`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PromptRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "messageID")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_reply: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<BTreeMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default)]
    pub parts: Vec<PromptPart>,
}

/// Prompt/message response shape. Message info and parts are intentionally
/// retained losslessly until the full upstream union generator is proven.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageWithParts {
    pub info: Value,
    #[serde(default)]
    pub parts: Vec<Value>,
}
