pub(super) fn supports_tool(tool_id: &str) -> bool {
    !default_packages(tool_id).is_empty()
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    // Arch ships psql in the client package, separately from the server.
    if tool_id == "psql" {
        return package_name == "postgresql-libs";
    }
    crate::backend::common::unix_family_matches(tool_id, package_name)
}

pub(super) fn default_packages(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        // Languages & runtimes
        "python" => &["python"],
        "node" => &["nodejs", "npm"],
        "typescript" => &["typescript"],
        "rust" => &["rust"],
        "go" => &["go"],
        "java" => &["jdk-openjdk"],
        "dotnet" => &["dotnet-sdk"],
        "ruby" => &["ruby"],
        "php" => &["php"],
        "zig" => &["zig"],
        "deno" => &["deno"],

        // Package managers
        "pnpm" => &["pnpm"],
        "yarn" => &["yarn"],
        "uv" => &["uv"],
        "pipx" => &["python-pipx"],
        "poetry" => &["python-poetry"],
        "composer" => &["composer"],

        // Version control
        "git" => &["git"],
        "git-lfs" => &["git-lfs"],
        "gh" => &["github-cli"],
        "glab" => &["glab"],
        "lazygit" => &["lazygit"],
        "delta" => &["git-delta"],

        // Build & compilers
        "cmake" => &["cmake"],
        "gcc" => &["gcc"],
        "clang" => &["clang"],
        "make" => &["make"],
        "ninja" => &["ninja"],
        "meson" => &["meson"],
        "maven" => &["maven"],
        "gdb" => &["gdb"],
        "just" => &["just"],
        "protoc" => &["protobuf"],

        // Containers & cloud native
        "docker" => &["docker"],
        "podman" => &["podman"],
        "kubectl" => &["kubectl"],
        "helm" => &["helm"],
        "terraform" => &["terraform"],
        "ansible" => &["ansible-core"],

        // Database clients
        "sqlite" => &["sqlite"],
        "psql" => &["postgresql-libs"],
        "mysql" => &["mariadb-clients"],
        "redis-cli" => &["valkey"],

        // Editors & terminal CLI
        "neovim" => &["neovim"],
        "vim" => &["vim"],
        "tmux" => &["tmux"],
        "ripgrep" => &["ripgrep"],
        "fd" => &["fd"],
        "bat" => &["bat"],
        "jq" => &["jq"],
        "fzf" => &["fzf"],
        "curl" => &["curl"],
        "httpie" => &["httpie"],
        "shellcheck" => &["shellcheck"],
        "shfmt" => &["shfmt"],
        "direnv" => &["direnv"],
        "starship" => &["starship"],
        "zoxide" => &["zoxide"],
        "eza" => &["eza"],
        _ => &[],
    }
}
