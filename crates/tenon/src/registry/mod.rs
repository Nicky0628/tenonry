use crate::error::{Error, Result};
use crate::model::{ToolChoice, ToolSpec};
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
    ToolSpec::new("python", "Python", "languages", &["python", "python3"]).versioned(),
    ToolSpec::new("node", "Node.js", "languages", &["node"]),
    ToolSpec::new("typescript", "TypeScript", "languages", &["tsc"]),
    ToolSpec::new("rust", "Rust", "languages", &["cargo", "rustc"])
        .require_all()
        .per_program(),
    ToolSpec::new("go", "Go", "languages", &["go"]).version_args(&["version"]),
    ToolSpec::new("java", "Java", "languages", &["java", "javac"])
        .version_args(&["-version"])
        .versioned()
        .per_program(),
    ToolSpec::new("dotnet", ".NET SDK", "languages", &["dotnet"]),
    ToolSpec::new("ruby", "Ruby", "languages", &["ruby"]).versioned(),
    ToolSpec::new("php", "PHP", "languages", &["php"]).versioned(),
    ToolSpec::new("zig", "Zig", "languages", &["zig"])
        .version_args(&["version"])
        .versioned(),
    ToolSpec::new("bun", "Bun", "languages", &["bun"]),
    ToolSpec::new("deno", "Deno", "languages", &["deno"]),
    // Package managers
    ToolSpec::new("pnpm", "pnpm", "package-managers", &["pnpm"]),
    ToolSpec::new("yarn", "Yarn", "package-managers", &["yarn", "yarnpkg"]),
    ToolSpec::new("uv", "uv", "package-managers", &["uv"]),
    ToolSpec::new("pipx", "pipx", "package-managers", &["pipx"]),
    ToolSpec::new("poetry", "Poetry", "package-managers", &["poetry"]),
    ToolSpec::new("composer", "Composer", "package-managers", &["composer"]),
    // Version control
    ToolSpec::new("git", "Git", "version-control", &["git"]),
    ToolSpec::new("git-lfs", "Git LFS", "version-control", &["git-lfs"]),
    ToolSpec::new("gh", "GitHub CLI", "version-control", &["gh"]),
    ToolSpec::new("glab", "GitLab CLI", "version-control", &["glab"]),
    ToolSpec::new("lazygit", "lazygit", "version-control", &["lazygit"]),
    ToolSpec::new("delta", "delta", "version-control", &["delta"]),
    // Build & compilers
    ToolSpec::new("cmake", "CMake", "build", &["cmake"]),
    ToolSpec::new("gcc", "GCC", "build", &["gcc"])
        .versioned()
        .linux_only(),
    ToolSpec::new("clang", "Clang", "build", &["clang"]).versioned(),
    ToolSpec::new("make", "GNU Make", "build", &["make"]),
    ToolSpec::new("ninja", "Ninja", "build", &["ninja"]),
    ToolSpec::new("meson", "Meson", "build", &["meson"]),
    ToolSpec::new("maven", "Apache Maven", "build", &["mvn"]),
    ToolSpec::new("gdb", "GDB", "build", &["gdb"]).linux_only(),
    ToolSpec::new("just", "just", "build", &["just"]),
    ToolSpec::new("protoc", "Protobuf", "build", &["protoc"]),
    // Containers & cloud native
    ToolSpec::new("docker", "Docker CLI", "containers", &["docker"]),
    ToolSpec::new("podman", "Podman", "containers", &["podman"]),
    ToolSpec::new("kubectl", "kubectl", "containers", &["kubectl"])
        .version_args(&["version", "--client"]),
    ToolSpec::new("helm", "Helm", "containers", &["helm"]).version_args(&["version", "--short"]),
    ToolSpec::new("terraform", "Terraform", "containers", &["terraform"])
        .version_args(&["version"]),
    ToolSpec::new("ansible", "Ansible", "containers", &["ansible"]).linux_only(),
    // Database clients
    ToolSpec::new("sqlite", "SQLite", "databases", &["sqlite3"]),
    ToolSpec::new("psql", "PostgreSQL Client", "databases", &["psql"]),
    ToolSpec::new(
        "mysql",
        "MySQL / MariaDB",
        "databases",
        &["mysql", "mariadb"],
    ),
    ToolSpec::new(
        "redis-cli",
        "Redis / Valkey CLI",
        "databases",
        &["redis-cli", "valkey-cli"],
    ),
    // Editors & terminal CLI
    ToolSpec::new("neovim", "Neovim", "cli", &["nvim"]),
    ToolSpec::new("vim", "Vim", "cli", &["vim"]),
    ToolSpec::new("tmux", "tmux", "cli", &["tmux"])
        .version_args(&["-V"])
        .linux_only(),
    ToolSpec::new("ripgrep", "ripgrep", "cli", &["rg"]),
    ToolSpec::new("fd", "fd", "cli", &["fd", "fdfind"]),
    ToolSpec::new("bat", "bat", "cli", &["bat", "batcat"]),
    ToolSpec::new("jq", "jq", "cli", &["jq"]),
    ToolSpec::new("fzf", "fzf", "cli", &["fzf"]),
    ToolSpec::new("curl", "curl", "cli", &["curl"]),
    ToolSpec::new("httpie", "HTTPie", "cli", &["http"]),
    ToolSpec::new("shellcheck", "ShellCheck", "cli", &["shellcheck"]),
    ToolSpec::new("shfmt", "shfmt", "cli", &["shfmt"]),
    ToolSpec::new("direnv", "direnv", "cli", &["direnv"]).linux_only(),
    ToolSpec::new("starship", "Starship", "cli", &["starship"]),
    ToolSpec::new("zoxide", "zoxide", "cli", &["zoxide"]),
    ToolSpec::new("eza", "eza", "cli", &["eza"]),
];
