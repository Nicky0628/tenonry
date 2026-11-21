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
        "gh" => &["github-cli"],
        "java" => &["jdk-openjdk"],
        "dotnet" => &["dotnet-sdk"],
        "cmake" => &["cmake"],
        "go" => &["go"],
        "gcc" => &["gcc"],
        "git-lfs" => &["git-lfs"],
        "node" => &["nodejs", "npm"],
        "python" => &["python"],
        "rust" => &["rust"],
        "pnpm" => &["pnpm"],
        "git" => &["git"],
        _ => &[],
    }
}
