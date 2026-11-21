use super::BackendId;
use serde::{Deserialize, Serialize};

/// A single installable version offered by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: String,
    pub label: String,
    pub current: bool,
}

/// Available versions queried for a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub tool_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendId>,
    pub versions: Vec<VersionInfo>,
    pub message: String,
}
