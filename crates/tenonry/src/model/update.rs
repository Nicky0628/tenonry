use super::{BackendId, Preview};
use serde::{Deserialize, Serialize};

/// Result status of checking updates for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum UpdateState {
    /// A newer version is available from the owning backend.
    Available,
    /// The installed version is already up to date.
    Current,
    /// Update checking failed or could not be completed.
    Failed,
}

/// Update check result for one tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    /// Registry identifier of the checked tool.
    pub tool_id: String,
    /// Update availability status.
    pub state: UpdateState,
    /// Backend queried for update information, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendId>,
    /// Preflight preview of the available update, if any.
    pub preview: Option<Preview>,
    /// Human-readable status summary.
    pub message: String,
    /// RFC 3339 UTC timestamp when this update check ran.
    pub checked_at: String,
    /// Whether this result was served from an in-memory cache.
    #[serde(default)]
    pub cached: bool,
}

/// Refresh metadata for a backend package catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    /// Backend whose package catalog was refreshed.
    pub backend: BackendId,
    /// RFC 3339 UTC timestamp when the catalog refresh completed.
    pub checked_at: String,
    /// Human-readable refresh summary.
    pub message: String,
}
