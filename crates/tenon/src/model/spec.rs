/// Whether a tool exists on every supported platform. GCC is Linux-only because
/// Windows builds use MSVC rather than a system GCC package.
#[derive(Clone, Copy)]
pub(crate) enum Platforms {
    All,
    Linux,
}

impl Platforms {
    pub(crate) fn available(self) -> bool {
        match self {
            Platforms::All => true,
            Platforms::Linux => cfg!(target_os = "linux"),
        }
    }
}

/// Alias deduplication scope. Cargo and rustc can be hard links to the same
/// rustup binary while selecting distinct subprograms.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Identity {
    Single,
    PerProgram,
}

/// Backend-agnostic description of a supported development tool.
pub(crate) struct ToolSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) category: &'static str,
    pub(crate) programs: &'static [&'static str],
    pub(crate) version_arguments: &'static [&'static str],
    pub(crate) require_all: bool,
    pub(crate) versioned: bool,
    pub(crate) identity: Identity,
    pub(crate) platforms: Platforms,
}
