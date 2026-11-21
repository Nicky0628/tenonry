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
    let version = version.split_once(':').map_or(version, |(_, rest)| rest);
    let start = version.find(|ch: char| ch.is_ascii_digit())?;
    let value = &version[start..];
    if let Some(update) = value
        .strip_prefix("8u")
        .or_else(|| value.strip_prefix("1.8.0_"))
    {
        let update: String = update.chars().take_while(char::is_ascii_digit).collect();
        if !update.is_empty() {
            return Some(format!("8.0.{update}"));
        }
    }
    let numbers: Vec<_> = version[start..]
        .split(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .next()?
        .split('.')
        .filter(|part| !part.is_empty())
        .take(3)
        .collect();
    if numbers.len() < 2 {
        return None;
    }
    Some(numbers.join("."))
}

pub(crate) const TOOLS: &[ToolSpec] = &[
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
];
