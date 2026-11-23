pub(super) fn supports_tool(tool_id: &str) -> bool {
    !default_packages(tool_id).is_empty()
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    crate::backend::common::unix_family_matches(tool_id, package_name)
}

pub(super) fn default_packages(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        // Languages & runtimes
        "python" => &["python3"],
        "node" => &["nodejs", "npm"],
        "rust" => &["rust", "cargo"],
        "go" => &["golang"],
        "java" => &["java-21-openjdk-devel"],
        "dotnet" => &["dotnet-sdk-8.0"],
        "ruby" => &["ruby"],
        "php" => &["php-cli"],
        "zig" => &["zig"],

        // Package managers
        "yarn" => &["yarnpkg"],
        "uv" => &["uv"],
        "pipx" => &["pipx"],
        "poetry" => &["poetry"],
        "composer" => &["composer"],

        // Version control
        "git" => &["git"],
        "git-lfs" => &["git-lfs"],
        "gh" => &["gh"],
        "glab" => &["glab"],
        "delta" => &["git-delta"],

        // Build & compilers
        "cmake" => &["cmake"],
        "gcc" => &["gcc", "gcc-c++"],
        "clang" => &["clang"],
        "make" => &["make"],
        "ninja" => &["ninja-build"],
        "meson" => &["meson"],
        "maven" => &["maven"],
        "gdb" => &["gdb"],
        "just" => &["just"],
        "protoc" => &["protobuf-compiler"],

        // Containers & cloud native
        "docker" => &["moby-engine"],
        "podman" => &["podman"],
        "kubectl" => &["kubernetes-client"],
        "helm" => &["helm"],
        "ansible" => &["ansible-core"],

        // Database clients
        "sqlite" => &["sqlite"],
        "psql" => &["postgresql"],
        "mysql" => &["mariadb"],
        "redis-cli" => &["valkey"],

        // Editors & terminal CLI
        "neovim" => &["neovim"],
        "vim" => &["vim-enhanced"],
        "tmux" => &["tmux"],
        "ripgrep" => &["ripgrep"],
        "fd" => &["fd-find"],
        "bat" => &["bat"],
        "jq" => &["jq"],
        "fzf" => &["fzf"],
        "curl" => &["curl"],
        "httpie" => &["httpie"],
        "shellcheck" => &["ShellCheck"],
        "shfmt" => &["shfmt"],
        "direnv" => &["direnv"],
        "zoxide" => &["zoxide"],
        "eza" => &["eza"],
        _ => &[],
    }
}
