pub(super) fn base_name(value: &str) -> &str {
    value.split(':').next().unwrap_or(value)
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "pnpm"
            | "git"
            | "git-lfs"
            | "gh"
            | "python"
            | "node"
            | "rust"
            | "go"
            | "java"
            | "dotnet"
            | "cmake"
            | "gcc"
    )
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    crate::backend::common::unix_family_matches(tool_id, package_name)
}

pub(super) fn default_packages(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        "java" => &["default-jdk"],
        "dotnet" => &["dotnet-sdk-8.0"],
        "cmake" => &["cmake"],
        "go" => &["golang-go"],
        "git-lfs" => &["git-lfs"],
        "gcc" => &["build-essential"],
        "node" => &["nodejs", "npm"],
        "python" => &["python3"],
        "rust" => &["rustc", "cargo"],
        "pnpm" => &["pnpm"],
        "git" => &["git"],
        "gh" => &["gh"],
        _ => &[],
    }
}

fn is_numeric_version(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

pub(super) fn version_query_prefix(tool_id: &str) -> &str {
    match tool_id {
        "java" => "openjdk-",
        "go" => "golang-",
        "rust" => "rustc",
        "node" => "nodejs",
        _ => tool_id,
    }
}

pub(super) fn is_primary_package(tool_id: &str, name: &str) -> bool {
    match tool_id {
        "java" => name
            .strip_prefix("openjdk-")
            .and_then(|s| s.strip_suffix("-jdk"))
            .is_some_and(is_numeric_version),
        "python" => name
            .strip_prefix("python")
            .is_some_and(|s| s.contains('.') && is_numeric_version(s)),
        "go" => name
            .strip_prefix("golang-")
            .and_then(|s| s.strip_suffix("-go"))
            .is_some_and(is_numeric_version),
        "dotnet" => name
            .strip_prefix("dotnet-sdk-")
            .is_some_and(is_numeric_version),
        "gcc" => name.strip_prefix("gcc-").is_some_and(is_numeric_version),
        "node" => name == "nodejs" || name.strip_prefix("nodejs").is_some_and(is_numeric_version),
        "rust" => name == "rustc",
        _ => name == tool_id,
    }
}

pub(super) fn package_series<'a>(tool_id: &str, package: &'a str) -> &'a str {
    match tool_id {
        "java" => package.strip_suffix("-jdk").unwrap_or(package),
        "go" => package.strip_suffix("-go").unwrap_or(package),
        _ => package,
    }
}

pub(super) fn belongs_to_series(tool_id: &str, name: &str, selected: &str) -> bool {
    match tool_id {
        "java" | "go" => {
            name == selected
                || name
                    .strip_prefix(selected)
                    .is_some_and(|s| s.starts_with('-'))
        }
        "python" => name == selected || name == format!("{selected}-minimal"),
        "gcc" => name == selected || name.starts_with(&format!("{selected}-")),
        "node" => name == selected || name == "npm",
        "rust" => name == selected || name == "cargo",
        "dotnet" => {
            name == "dotnet-host"
                || name.ends_with(selected.strip_prefix("dotnet-sdk-").unwrap_or(selected))
        }
        _ => name == selected,
    }
}
