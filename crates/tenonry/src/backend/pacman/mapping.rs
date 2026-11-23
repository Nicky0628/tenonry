const SAME_NAME: &[&str] = &[
    "python",
    "typescript",
    "rust",
    "go",
    "ruby",
    "php",
    "zig",
    "deno",
    "pnpm",
    "yarn",
    "uv",
    "composer",
    "git",
    "git-lfs",
    "glab",
    "lazygit",
    "cmake",
    "gcc",
    "clang",
    "make",
    "ninja",
    "meson",
    "maven",
    "gdb",
    "just",
    "docker",
    "podman",
    "kubectl",
    "helm",
    "terraform",
    "sqlite",
    "neovim",
    "vim",
    "tmux",
    "ripgrep",
    "fd",
    "bat",
    "jq",
    "fzf",
    "curl",
    "httpie",
    "shellcheck",
    "shfmt",
    "direnv",
    "starship",
    "zoxide",
    "eza",
];

fn custom_packages(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        "node" => &["nodejs", "npm"],
        "java" => &["jdk-openjdk"],
        "dotnet" => &["dotnet-sdk"],
        "pipx" => &["python-pipx"],
        "poetry" => &["python-poetry"],
        "gh" => &["github-cli"],
        "delta" => &["git-delta"],
        "protoc" => &["protobuf"],
        "ansible" => &["ansible-core"],
        "psql" => &["postgresql-libs"],
        "mysql" => &["mariadb-clients"],
        "redis-cli" => &["valkey"],
        _ => &[],
    }
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    SAME_NAME.contains(&tool_id) || !custom_packages(tool_id).is_empty()
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    // Arch ships psql in the client package, separately from the server.
    if tool_id == "psql" {
        return package_name == "postgresql-libs";
    }
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
