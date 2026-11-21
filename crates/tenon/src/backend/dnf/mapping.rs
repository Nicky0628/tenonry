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
        "java" => &["java-21-openjdk-devel"],
        "cmake" => &["cmake"],
        "go" => &["golang"],
        "git-lfs" => &["git-lfs"],
        "gcc" => &["gcc", "gcc-c++"],
        "dotnet" => &["dotnet-sdk-8.0"],
        "node" => &["nodejs", "npm"],
        "python" => &["python3"],
        "rust" => &["rust", "cargo"],
        "pnpm" => &["pnpm"],
        "git" => &["git"],
        "gh" => &["gh"],
        _ => &[],
    }
}
