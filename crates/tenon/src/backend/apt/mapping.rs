const SAME_NAME: &[&str] = &[
    "ruby",
    "zig",
    "pipx",
    "composer",
    "git",
    "git-lfs",
    "gh",
    "glab",
    "lazygit",
    "cmake",
    "clang",
    "make",
    "meson",
    "maven",
    "gdb",
    "just",
    "podman",
    "neovim",
    "vim",
    "tmux",
    "ripgrep",
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
        "python" => &["python3"],
        "node" => &["nodejs", "npm"],
        "typescript" => &["node-typescript"],
        "rust" => &["rustc", "cargo"],
        "go" => &["golang-go"],
        "java" => &["default-jdk"],
        "dotnet" => &["dotnet-sdk-8.0"],
        "php" => &["php-cli"],
        "yarn" => &["yarnpkg"],
        "poetry" => &["python3-poetry"],
        "delta" => &["git-delta"],
        "gcc" => &["build-essential"],
        "ninja" => &["ninja-build"],
        "protoc" => &["protobuf-compiler"],
        "docker" => &["docker.io"],
        "ansible" => &["ansible-core"],
        "sqlite" => &["sqlite3"],
        "psql" => &["postgresql-client"],
        "mysql" => &["default-mysql-client"],
        "redis-cli" => &["redis-tools"],
        "fd" => &["fd-find"],
        _ => &[],
    }
}

pub(super) fn base_name(value: &str) -> &str {
    value.split(':').next().unwrap_or(value)
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    SAME_NAME.contains(&tool_id) || !custom_packages(tool_id).is_empty()
}

pub(super) fn matches_tool(tool_id: &str, package_name: &str) -> bool {
    if tool_id == "psql" {
        let name = base_name(package_name);
        return matches!(name, "postgresql-client" | "postgresql-client-common")
            || name
                .strip_prefix("postgresql-client-")
                .is_some_and(is_numeric_version);
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

fn primary_single(tool_id: &str) -> &str {
    custom_packages(tool_id).first().copied().unwrap_or(tool_id)
}

fn is_numeric_version(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

pub(super) fn version_query_prefixes(tool_id: &str) -> Vec<&str> {
    match tool_id {
        "python" => vec!["python"],
        "java" => vec!["openjdk-"],
        "go" => vec!["golang-"],
        "rust" => vec!["rustc"],
        "node" => vec!["nodejs"],
        "gcc" => vec!["gcc-"],
        "clang" => vec!["clang-"],
        "dotnet" => vec!["dotnet-sdk-"],
        "ruby" => vec!["ruby"],
        "php" => vec!["php"],
        "zig" => vec!["zig"],
        "psql" => vec!["postgresql-client-"],
        "mysql" => vec!["mysql-client", "mariadb-client"],
        "docker" => vec!["docker.io", "docker-ce-cli", "docker-cli"],
        _ => vec![primary_single(tool_id)],
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
        "clang" => name.strip_prefix("clang-").is_some_and(is_numeric_version),
        "node" => name == "nodejs" || name.strip_prefix("nodejs").is_some_and(is_numeric_version),
        "rust" => name == "rustc",
        "ruby" => name
            .strip_prefix("ruby")
            .is_some_and(|s| s.contains('.') && is_numeric_version(s)),
        "php" => name
            .strip_prefix("php")
            .and_then(|s| s.strip_suffix("-cli"))
            .is_some_and(|s| s.contains('.') && is_numeric_version(s)),
        "zig" => name
            .strip_prefix("zig")
            .is_some_and(|s| s.contains('.') && is_numeric_version(s)),
        "psql" => name
            .strip_prefix("postgresql-client-")
            .is_some_and(is_numeric_version),
        "mysql" => matches!(name, "mysql-client" | "mariadb-client"),
        "docker" => matches!(name, "docker.io" | "docker-ce-cli" | "docker-cli"),
        _ => name == primary_single(tool_id),
    }
}

pub(super) fn package_series<'a>(tool_id: &str, package: &'a str) -> &'a str {
    match tool_id {
        "java" => package.strip_suffix("-jdk").unwrap_or(package),
        "go" => package.strip_suffix("-go").unwrap_or(package),
        "php" => package.strip_suffix("-cli").unwrap_or(package),
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
        "gcc" | "clang" => name == selected || name.starts_with(&format!("{selected}-")),
        "ruby" => name == selected || name == "ruby",
        "php" => {
            name == selected
                || name == format!("{selected}-cli")
                || matches!(name, "php" | "php-cli")
        }
        "zig" => name == selected || name == format!("{selected}-dev"),
        "node" => name == selected || name == "npm",
        "rust" => name == selected || name == "cargo",
        "dotnet" => {
            name == "dotnet-host"
                || name.ends_with(selected.strip_prefix("dotnet-sdk-").unwrap_or(selected))
        }
        "psql" => {
            name == selected || matches!(name, "postgresql-client" | "postgresql-client-common")
        }
        "mysql" => {
            name == "default-mysql-client"
                || name == selected
                || name.starts_with(&format!("{selected}-"))
        }
        _ => name == selected,
    }
}
