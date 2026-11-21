use serde::{Deserialize, Serialize};
use std::fmt;

/// Scope of a package manager backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendScope {
    System,
    Ecosystem,
}

/// Supported package manager backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendId {
    Apt,
    Dnf,
    Pacman,
    Winget,
    Npm,
}

/// Explicit feature matrix declared by each backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendCapabilities {
    pub scope: BackendScope,
    pub version_selection: bool,
    pub batch_transaction: bool,
    pub catalog_refresh: bool,
    pub isolated_catalog: bool,
}

/// Discovery status of a backend on the current machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub id: BackendId,
    pub name: String,
    pub capabilities: BackendCapabilities,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

impl BackendId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Winget => "winget",
            Self::Npm => "npm",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Apt => "APT",
            Self::Dnf => "DNF",
            Self::Pacman => "pacman",
            Self::Winget => "WinGet",
            Self::Npm => "npm",
        }
    }

    pub const fn capabilities(self) -> BackendCapabilities {
        match self {
            Self::Apt => BackendCapabilities {
                scope: BackendScope::System,
                version_selection: true,
                batch_transaction: true,
                catalog_refresh: true,
                isolated_catalog: true,
            },
            Self::Dnf => BackendCapabilities {
                scope: BackendScope::System,
                version_selection: false,
                batch_transaction: true,
                catalog_refresh: true,
                isolated_catalog: false,
            },
            Self::Pacman => BackendCapabilities {
                scope: BackendScope::System,
                version_selection: false,
                batch_transaction: true,
                catalog_refresh: true,
                isolated_catalog: false,
            },
            Self::Winget => BackendCapabilities {
                scope: BackendScope::System,
                version_selection: false,
                batch_transaction: true,
                catalog_refresh: true,
                isolated_catalog: false,
            },
            Self::Npm => BackendCapabilities {
                scope: BackendScope::Ecosystem,
                version_selection: true,
                batch_transaction: false,
                catalog_refresh: false,
                isolated_catalog: false,
            },
        }
    }

    pub const fn scope(self) -> BackendScope {
        self.capabilities().scope
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}
