use super::{Action, BackendId, BackendStatus};
use crate::error::Error;
use serde::{Deserialize, Serialize};

/// Metadata for a supported development tool in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoice {
    /// Stable registry identifier (for example, `"node"` or `"python"`).
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Category label used for grouping in UIs.
    pub category: &'static str,
}

/// Lightweight command availability status on `PATH`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    /// Stable registry identifier.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Whether at least one matching executable was found on `PATH`.
    pub found: bool,
    /// Detected version string, if probing succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional probe hint or diagnostic note, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Lightweight probe report without querying package manager inventories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    /// Per-tool `PATH` probe results.
    pub tools: Vec<ToolStatus>,
    /// RFC 3339 UTC timestamp when the probe completed.
    pub checked_at: String,
}

/// Ownership classification of a discovered executable file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ExecutableSource {
    /// Owned by a recognized package manager backend.
    Backend {
        /// Owning package manager backend.
        backend: BackendId,
    },
    /// Bundled as part of another parent tool (for example, `npm` bundled with Node.js).
    Bundled,
    /// Managed by an external version manager (such as `nvm`, `pyenv`, `rustup`, or `asdf`).
    VersionManager,
    /// Installed outside supported package managers (for example, manual binary or installer).
    External,
}

/// A discovered executable entry for a development tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Executable {
    /// Original path discovered on `PATH` or in known backend locations.
    pub path: String,
    /// Canonicalized or symlink-resolved filesystem path.
    pub resolved_path: String,
    /// Command or executable name that matched.
    pub program: String,
    /// Version reported by this executable, if detected.
    pub version: Option<String>,
    /// Whether this executable is currently reachable on `PATH`.
    pub on_path: bool,
    /// Active command names resolving to this executable on `PATH`.
    #[serde(default)]
    pub active_programs: Vec<String>,
    /// Attributed installation source for this executable.
    pub source: ExecutableSource,
    /// Diagnostic warning specific to this executable, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

/// An installed package record reported by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    /// Backend-specific package name.
    pub name: String,
    /// Installed package version string.
    pub version: String,
}

/// High-level lifecycle state of a tool on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ToolState {
    /// Installed and managed cleanly by a recognized backend or bundle.
    Installed,
    /// Not currently installed on the system.
    Missing,
    /// Present on the system outside supported package managers.
    External,
    /// Present as a system-protected component that cannot be altered automatically.
    System,
    /// Cannot be managed due to missing backends or conflicting ownership.
    Unavailable,
}

/// Effective installation source summary for a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ToolSource {
    /// Cleanly managed by a single package manager backend.
    Managed {
        /// Owning package manager backend.
        backend: BackendId,
    },
    /// Installed by a backend alongside conflicting external or version-manager binaries.
    Mixed {
        /// Owning package manager backend.
        backend: BackendId,
    },
    /// Installed simultaneously across more than one package manager backend.
    MultiBackend {
        /// Conflicting package manager backends.
        backends: Vec<BackendId>,
    },
    /// Provided as a bundled component of another tool.
    Bundled,
    /// Installed exclusively outside supported package managers.
    External,
    /// Not installed and no supported backend is available to install it.
    Unavailable,
}

/// Full inspection report for a single development tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolReport {
    /// Lightweight `PATH` probe status for this tool.
    #[serde(flatten)]
    pub status: ToolStatus,
    /// High-level lifecycle state.
    pub state: ToolState,
    /// Attributed installation source summary.
    pub source: ToolSource,
    /// Owning backend when managed unambiguously by a single backend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<BackendId>,
    /// Available backends on this machine capable of installing or managing this tool.
    #[serde(default)]
    pub available_backends: Vec<BackendId>,
    /// All executables discovered for this tool.
    pub executables: Vec<Executable>,
    /// Installed package records associated with this tool.
    pub packages: Vec<Package>,
    /// Lifecycle actions currently permitted for this tool.
    pub actions: Vec<Action>,
    /// Whether callers can choose an explicit version for this tool.
    #[serde(default)]
    pub can_select_version: bool,
    /// Human-readable status explanation or restriction notice.
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
    /// Full inspection reports for requested or all platform-supported tools.
    pub tools: Vec<ToolReport>,
    /// Discovery status for all candidate backends on this OS.
    pub backends: Vec<BackendStatus>,
    /// RFC 3339 UTC timestamp when inspection completed.
    pub checked_at: String,
    /// Non-fatal environment warning encountered during inspection, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}
