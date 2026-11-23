/// Whether a tool exists on every supported platform or only Linux (e.g. GCC, GDB, tmux, Ansible, direnv).
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

impl ToolSpec {
    pub(crate) const fn new(
        id: &'static str,
        name: &'static str,
        category: &'static str,
        programs: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            category,
            programs,
            version_arguments: &["--version"],
            require_all: false,
            versioned: false,
            identity: Identity::Single,
            platforms: Platforms::All,
        }
    }

    pub(crate) const fn version_args(mut self, args: &'static [&'static str]) -> Self {
        self.version_arguments = args;
        self
    }

    pub(crate) const fn require_all(mut self) -> Self {
        self.require_all = true;
        self
    }

    pub(crate) const fn versioned(mut self) -> Self {
        self.versioned = true;
        self
    }

    pub(crate) const fn per_program(mut self) -> Self {
        self.identity = Identity::PerProgram;
        self
    }

    pub(crate) const fn linux_only(mut self) -> Self {
        self.platforms = Platforms::Linux;
        self
    }
}
