use serde::{Deserialize, Serialize};
use std::fmt;

/// Scope of a package manager backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum BackendScope {
    /// OS-level system package manager (such as APT, DNF, pacman, or WinGet).
    System,
    /// Language or ecosystem package manager (such as npm).
    Ecosystem,
}

/// Supported package manager backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum BackendId {
    /// Debian/Ubuntu APT (`apt-get`, `apt-cache`, `dpkg-query`).
    Apt,
    /// Fedora/RHEL DNF (`dnf`, `rpm`).
    Dnf,
    /// Arch Linux pacman (`pacman`).
    Pacman,
    /// Windows Package Manager (`winget`).
    Winget,
    /// Node.js global package manager (`npm`).
    Npm,
}

/// Explicit feature matrix declared by each backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendCapabilities {
    /// Whether this backend operates at the OS level or within a language ecosystem.
    pub scope: BackendScope,
    /// Whether callers may target a specific version during install or update.
    pub version_selection: bool,
    /// Whether multiple tool updates can be merged into one backend transaction.
    pub batch_transaction: bool,
    /// Whether this backend supports explicit package catalog refresh.
    pub catalog_refresh: bool,
    /// Whether catalog refresh writes to an isolated cache rather than global OS state.
    pub isolated_catalog: bool,
}

/// Discovery status of a backend on the current machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    /// Backend identifier.
    pub id: BackendId,
    /// Human-readable backend display name.
    pub name: String,
    /// Declared capability flags for this backend.
    pub capabilities: BackendCapabilities,
    /// Whether the backend executable and environment are usable on this machine.
    pub available: bool,
    /// Diagnostic explanation when the backend is unavailable or degraded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

impl BackendId {
    /// Returns the stable lowercase identifier string for this backend.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Winget => "winget",
            Self::Npm => "npm",
        }
    }

    /// Returns the human-readable display name for this backend.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Apt => "APT",
            Self::Dnf => "DNF",
            Self::Pacman => "pacman",
            Self::Winget => "WinGet",
            Self::Npm => "npm",
        }
    }

    /// Returns the static capability matrix for this backend.
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

    /// Returns the [`BackendScope`] declared in [`Self::capabilities`].
    pub const fn scope(self) -> BackendScope {
        self.capabilities().scope
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}
