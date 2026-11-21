use super::{BackendId, Preview};
use serde::{Deserialize, Serialize};

/// Result status of checking updates for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateState {
    Available,
    Current,
    Failed,
}

/// Update check result for one tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub tool_id: String,
    pub state: UpdateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendId>,
    pub preview: Option<Preview>,
    pub message: String,
    pub checked_at: String,
    #[serde(default)]
    pub cached: bool,
}

/// Refresh metadata for a backend package catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    pub backend: BackendId,
    pub checked_at: String,
    pub message: String,
}
