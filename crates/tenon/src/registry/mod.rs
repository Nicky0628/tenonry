use crate::error::{Error, Result};
use crate::model::{Identity, Platforms, ToolChoice, ToolSpec};
use std::collections::BTreeSet;

/// Lists the tools supported on the current platform, in display order.
pub fn available_tools() -> Vec<ToolChoice> {
    TOOLS
        .iter()
        .filter(|spec| spec.platforms.available())
        .map(|spec| ToolChoice {
            id: spec.id,
            name: spec.name,
            category: spec.category,
        })
        .collect()
}

/// Recognizes persisted tool IDs even when unavailable on the current platform.
pub fn is_known_tool(id: &str) -> bool {
    TOOLS.iter().any(|tool| tool.id == id)
}

pub(crate) fn find_available(id: &str) -> Result<&'static ToolSpec> {
    TOOLS
        .iter()
        .find(|spec| spec.id == id && spec.platforms.available())
        .ok_or_else(|| Error::not_found("请选择有效的开发工具。"))
}

pub(crate) fn requested(ids: Option<&[String]>) -> Result<Vec<&'static ToolSpec>> {
    if let Some(ids) = ids {
        let mut seen = BTreeSet::new();
        if ids.iter().any(|id| !is_known_tool(id) || !seen.insert(id)) {
            return Err(Error::invalid_input("工具标识无效或重复。"));
        }
    }
    Ok(TOOLS
        .iter()
        .filter(|spec| spec.platforms.available())
        .filter(|spec| ids.is_none_or(|ids| ids.iter().any(|id| id == spec.id)))
        .collect())
}

pub(crate) fn max_tool_count() -> usize {
    TOOLS.len()
}

/// Extracts the upstream release triplet from a package or CLI version string.
pub(crate) fn extract_release(version: &str) -> Option<String> {
    let version = match version.split_once(':') {
        Some((epoch, rest)) if !epoch.is_empty() && epoch.chars().all(|ch| ch.is_ascii_digit()) => {
            rest
        }
        _ => version,
    };

    if let Some(pos) = version.find("1.8.0_").or_else(|| version.find("8u")) {
        let tail = &version[pos..];
        let update = tail
            .strip_prefix("1.8.0_")
            .or_else(|| tail.strip_prefix("8u"))
            .unwrap_or("");
        let digits: String = update.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return Some(format!("8.0.{digits}"));
        }
    }

    for token in version.split(|ch: char| !ch.is_ascii_digit() && ch != '.') {
        let numbers: Vec<_> = token
            .split('.')
            .filter(|part| !part.is_empty())
            .take(3)
            .collect();
        if numbers.len() >= 2 {
            return Some(numbers.join("."));
        }
    }
    None
}

/// Compares two extracted releases, allowing a 2-part CLI version (e.g. `9.1`)
/// to match a 3-part package release from the same minor line (e.g. `9.1.2141`).
pub(crate) fn releases_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let prefix_match = |short: &str, long: &str| {
        short.matches('.').count() == 1
            && long
                .strip_prefix(short)
                .is_some_and(|rest| rest.starts_with('.'))
    };
    prefix_match(left, right) || prefix_match(right, left)
}

pub(crate) const TOOLS: &[ToolSpec] = &[
    // Languages & runtimes
    ToolSpec {
        id: "python",
        name: "Python",
        category: "languages",
        programs: &["python", "python3"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "node",
        name: "Node.js",
        category: "languages",
        programs: &["node"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "typescript",
        name: "TypeScript",
        category: "languages",
        programs: &["tsc"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "rust",
        name: "Rust",
        category: "languages",
        programs: &["cargo", "rustc"],
        version_arguments: &["--version"],
        require_all: true,
        versioned: false,
        identity: Identity::PerProgram,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "go",
        name: "Go",
        category: "languages",
        programs: &["go"],
        version_arguments: &["version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "java",
        name: "Java",
        category: "languages",
        programs: &["java", "javac"],
        version_arguments: &["-version"],
        require_all: false,
        versioned: true,
        identity: Identity::PerProgram,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "dotnet",
        name: ".NET SDK",
        category: "languages",
        programs: &["dotnet"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "ruby",
        name: "Ruby",
        category: "languages",
        programs: &["ruby"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "php",
        name: "PHP",
        category: "languages",
        programs: &["php"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "zig",
        name: "Zig",
        category: "languages",
        programs: &["zig"],
        version_arguments: &["version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "bun",
        name: "Bun",
        category: "languages",
        programs: &["bun"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "deno",
        name: "Deno",
        category: "languages",
        programs: &["deno"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    // Package managers
    ToolSpec {
        id: "pnpm",
        name: "pnpm",
        category: "package-managers",
        programs: &["pnpm"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "yarn",
        name: "Yarn",
        category: "package-managers",
        programs: &["yarn", "yarnpkg"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "uv",
        name: "uv",
        category: "package-managers",
        programs: &["uv"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "pipx",
        name: "pipx",
        category: "package-managers",
        programs: &["pipx"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "poetry",
        name: "Poetry",
        category: "package-managers",
        programs: &["poetry"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "composer",
        name: "Composer",
        category: "package-managers",
        programs: &["composer"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    // Version control
    ToolSpec {
        id: "git",
        name: "Git",
        category: "version-control",
        programs: &["git"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "git-lfs",
        name: "Git LFS",
        category: "version-control",
        programs: &["git-lfs"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "gh",
        name: "GitHub CLI",
        category: "version-control",
        programs: &["gh"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "glab",
        name: "GitLab CLI",
        category: "version-control",
        programs: &["glab"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "lazygit",
        name: "lazygit",
        category: "version-control",
        programs: &["lazygit"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "delta",
        name: "delta",
        category: "version-control",
        programs: &["delta"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    // Build & compilers
    ToolSpec {
        id: "cmake",
        name: "CMake",
        category: "build",
        programs: &["cmake"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "gcc",
        name: "GCC",
        category: "build",
        programs: &["gcc"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::Linux,
    },
    ToolSpec {
        id: "clang",
        name: "Clang",
        category: "build",
        programs: &["clang"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: true,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "make",
        name: "GNU Make",
        category: "build",
        programs: &["make"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "ninja",
        name: "Ninja",
        category: "build",
        programs: &["ninja"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "meson",
        name: "Meson",
        category: "build",
        programs: &["meson"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "maven",
        name: "Apache Maven",
        category: "build",
        programs: &["mvn"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "gdb",
        name: "GDB",
        category: "build",
        programs: &["gdb"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::Linux,
    },
    ToolSpec {
        id: "just",
        name: "just",
        category: "build",
        programs: &["just"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "protoc",
        name: "Protobuf",
        category: "build",
        programs: &["protoc"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    // Containers & cloud native
    ToolSpec {
        id: "docker",
        name: "Docker CLI",
        category: "containers",
        programs: &["docker"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "podman",
        name: "Podman",
        category: "containers",
        programs: &["podman"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "kubectl",
        name: "kubectl",
        category: "containers",
        programs: &["kubectl"],
        version_arguments: &["version", "--client"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "helm",
        name: "Helm",
        category: "containers",
        programs: &["helm"],
        version_arguments: &["version", "--short"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "terraform",
        name: "Terraform",
        category: "containers",
        programs: &["terraform"],
        version_arguments: &["version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "ansible",
        name: "Ansible",
        category: "containers",
        programs: &["ansible"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::Linux,
    },
    // Database clients
    ToolSpec {
        id: "sqlite",
        name: "SQLite",
        category: "databases",
        programs: &["sqlite3"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "psql",
        name: "PostgreSQL Client",
        category: "databases",
        programs: &["psql"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "mysql",
        name: "MySQL / MariaDB",
        category: "databases",
        programs: &["mysql", "mariadb"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "redis-cli",
        name: "Redis / Valkey CLI",
        category: "databases",
        programs: &["redis-cli", "valkey-cli"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    // Editors & terminal CLI
    ToolSpec {
        id: "neovim",
        name: "Neovim",
        category: "cli",
        programs: &["nvim"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "vim",
        name: "Vim",
        category: "cli",
        programs: &["vim"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "tmux",
        name: "tmux",
        category: "cli",
        programs: &["tmux"],
        version_arguments: &["-V"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::Linux,
    },
    ToolSpec {
        id: "ripgrep",
        name: "ripgrep",
        category: "cli",
        programs: &["rg"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "fd",
        name: "fd",
        category: "cli",
        programs: &["fd", "fdfind"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "bat",
        name: "bat",
        category: "cli",
        programs: &["bat", "batcat"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "jq",
        name: "jq",
        category: "cli",
        programs: &["jq"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "fzf",
        name: "fzf",
        category: "cli",
        programs: &["fzf"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "curl",
        name: "curl",
        category: "cli",
        programs: &["curl"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "httpie",
        name: "HTTPie",
        category: "cli",
        programs: &["http"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "shellcheck",
        name: "ShellCheck",
        category: "cli",
        programs: &["shellcheck"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "shfmt",
        name: "shfmt",
        category: "cli",
        programs: &["shfmt"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "direnv",
        name: "direnv",
        category: "cli",
        programs: &["direnv"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::Linux,
    },
    ToolSpec {
        id: "starship",
        name: "Starship",
        category: "cli",
        programs: &["starship"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "zoxide",
        name: "zoxide",
        category: "cli",
        programs: &["zoxide"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
    ToolSpec {
        id: "eza",
        name: "eza",
        category: "cli",
        programs: &["eza"],
        version_arguments: &["--version"],
        require_all: false,
        versioned: false,
        identity: Identity::Single,
        platforms: Platforms::All,
    },
];
