use super::BackendId;
use serde::{Deserialize, Serialize};

/// A single installable version offered by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    /// Exact version string accepted by [`crate::PlanInput::version`].
    pub version: String,
    /// Human-readable label shown in version pickers.
    pub label: String,
    /// Whether this version matches the currently installed version.
    pub current: bool,
}

/// Available versions queried for a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    /// Registry identifier of the queried tool.
    pub tool_id: String,
    /// Backend that supplied this version list, when resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendId>,
    /// Candidate versions ordered for display.
    pub versions: Vec<VersionInfo>,
    /// Human-readable status note or empty-list explanation.
    pub message: String,
}
