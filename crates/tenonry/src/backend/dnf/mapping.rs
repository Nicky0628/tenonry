const SAME_NAME: &[&str] = &[
    "ruby", "zig", "uv", "pipx", "poetry", "composer", "git", "git-lfs", "gh", "glab", "cmake",
    "clang", "make", "meson", "maven", "gdb", "just", "podman", "helm", "sqlite", "neovim", "tmux",
    "ripgrep", "bat", "jq", "fzf", "curl", "httpie", "shfmt", "direnv", "zoxide", "eza",
];

fn custom_packages(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        "python" => &["python3"],
        "node" => &["nodejs", "npm"],
        "rust" => &["rust", "cargo"],
        "go" => &["golang"],
        "java" => &["java-21-openjdk-devel"],
        "dotnet" => &["dotnet-sdk-8.0"],
        "php" => &["php-cli"],
        "yarn" => &["yarnpkg"],
        "delta" => &["git-delta"],
        "gcc" => &["gcc", "gcc-c++"],
        "ninja" => &["ninja-build"],
        "protoc" => &["protobuf-compiler"],
        "docker" => &["moby-engine"],
        "kubectl" => &["kubernetes-client"],
        "ansible" => &["ansible-core"],
        "psql" => &["postgresql"],
        "mysql" => &["mariadb"],
        "redis-cli" => &["valkey"],
        "vim" => &["vim-enhanced"],
        "fd" => &["fd-find"],
        "shellcheck" => &["ShellCheck"],
        _ => &[],
    }
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    SAME_NAME.contains(&tool_id) || !custom_packages(tool_id).is_empty()
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    crate::backend::common::unix_family_matches(tool_id, package_name)
}

pub(super) fn default_packages(tool_id: &str) -> Vec<String> {
    let custom = custom_packages(tool_id);
    if !custom.is_empty() {
        custom.iter().map(|pkg| (*pkg).into()).collect()
    } else if SAME_NAME.contains(&tool_id) {
        vec![tool_id.into()]
    } else {
        Vec::new()
    }
}
