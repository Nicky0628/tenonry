use super::{Action, BackendId, BackendStatus};
use crate::error::Error;
use serde::{Deserialize, Serialize};

/// Metadata for a supported development tool in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoice {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
}

/// Lightweight command availability status on `PATH`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Lightweight probe report without querying package manager inventories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    pub tools: Vec<ToolStatus>,
    pub checked_at: String,
}

/// Ownership classification of a discovered executable file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExecutableSource {
    Backend { backend: BackendId },
    Bundled,
    VersionManager,
    External,
}

/// A discovered executable entry for a development tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Executable {
    pub path: String,
    pub resolved_path: String,
    pub program: String,
    pub version: Option<String>,
    pub on_path: bool,
    #[serde(default)]
    pub active_programs: Vec<String>,
    pub source: ExecutableSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

/// An installed package record reported by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
}

/// High-level lifecycle state of a tool on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolState {
    Installed,
    Missing,
    External,
    System,
    Unavailable,
}

/// Effective installation source summary for a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ToolSource {
    Managed { backend: BackendId },
    Mixed { backend: BackendId },
    MultiBackend { backends: Vec<BackendId> },
    Bundled,
    External,
    Unavailable,
}

/// Full inspection report for a single development tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolReport {
    #[serde(flatten)]
    pub status: ToolStatus,
    pub state: ToolState,
    pub source: ToolSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<BackendId>,
    #[serde(default)]
    pub available_backends: Vec<BackendId>,
    pub executables: Vec<Executable>,
    pub packages: Vec<Package>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub can_select_version: bool,
    pub message: String,
}

impl ToolReport {
    pub(crate) fn action_error(&self) -> String {
        if self.message.is_empty() {
            "此工具当前不支持该操作。".into()
        } else {
            self.message.clone()
        }
    }

    pub(crate) fn action_rejection(&self, auth_issue: Option<String>) -> Error {
        if self.state == ToolState::System {
            return Error::protected(self.action_error());
        }
        if let Some(issue) = auth_issue {
            return Error::authorization(issue);
        }
        if matches!(
            self.source,
            ToolSource::MultiBackend { .. } | ToolSource::Mixed { .. }
        ) {
            return Error::conflict(self.action_error());
        }
        if self.state == ToolState::Unavailable && self.source == ToolSource::Unavailable {
            return Error::conflict(self.action_error());
        }
        Error::unsupported(self.action_error())
    }
}

/// Full system environment report including backend discovery status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemReport {
    pub tools: Vec<ToolReport>,
    pub backends: Vec<BackendStatus>,
    pub checked_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}
