use crate::error::{Error, Result};
use crate::model::{Change, Package, Preview, PreviewState};

pub(crate) fn is_valid_package_name(name: &str) -> bool {
    name.starts_with(|ch: char| ch.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_.+:@".contains(ch))
}

pub(crate) fn validate_package_names(packages: &[String]) -> Result<()> {
    if packages.is_empty() || packages.iter().any(|name| !is_valid_package_name(name)) {
        return Err(Error::invalid_input("没有可以操作的软件包。"));
    }
    Ok(())
}

pub(crate) fn validate_preview_limits(preview: &Preview) -> Result<()> {
    if preview.details.len() > 48 * 1024 || preview.changes.len() > 256 {
        return Err(Error::conflict(
            "依赖变化过多，无法完整展示，请通过原包管理器审阅。",
        ));
    }
    Ok(())
}

pub(crate) fn installed_version(installed: &[Package], name: &str) -> Option<String> {
    installed
        .iter()
        .find(|package| package.name == name)
        .map(|package| package.version.clone())
}

pub(crate) fn uninstall_preview(installed: &[Package], names: &[String]) -> Preview {
    let targets: Vec<_> = names
        .iter()
        .map(|name| Change {
            name: name.clone(),
            installed: installed_version(installed, name),
            candidate: None,
            dependency: false,
        })
        .collect();
    Preview {
        state: PreviewState::Available,
        basis: "本机安装记录".into(),
        changes: targets.clone(),
        targets,
        details: String::new(),
    }
}

pub(crate) fn unchanged_targets(
    installed: &[Package],
    names: &[String],
) -> std::collections::BTreeMap<String, Result<Change>> {
    names
        .iter()
        .map(|name| {
            let version = installed_version(installed, name);
            let target = version
                .as_ref()
                .ok_or_else(|| Error::not_found(format!("未找到 {name} 的安装记录。")))
                .map(|_| Change {
                    name: name.clone(),
                    installed: version.clone(),
                    candidate: version.clone(),
                    dependency: false,
                });
            (name.clone(), target)
        })
        .collect()
}

/// Shared Linux package-family matcher used by APT, DNF, and pacman.
pub(crate) fn unix_family_matches(tool_id: &str, package_name: &str) -> bool {
    let name = package_name.split(':').next().unwrap_or(package_name);
    let numeric_version =
        |value: &str| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit() || ch == '.');
    match tool_id {
        // Languages & runtimes
        "python" => {
            let name = name.strip_suffix("-minimal").unwrap_or(name);
            name == "python"
                || name.strip_prefix("python").is_some_and(|rest| {
                    let rest = rest.trim_start_matches('@');
                    numeric_version(rest)
                })
        }
        "node" => {
            matches!(name, "nodejs" | "node" | "npm")
                || name.starts_with("node@")
                || name.starts_with("nodejs-lts-")
                || name.strip_prefix("nodejs").is_some_and(|rest| {
                    !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit())
                })
        }
        "typescript" => matches!(name, "typescript" | "node-typescript" | "nodejs-typescript"),
        "rust" => matches!(name, "rust" | "rustc" | "cargo" | "rustup"),
        "go" => {
            name == "go"
                || name == "golang"
                || (name.starts_with("golang-") && name.ends_with("-go"))
        }
        "java" => {
            name != "java-common"
                && !name.ends_with("-doc")
                && !name.ends_with("-src")
                && (name.starts_with("openjdk")
                    || name.starts_with("temurin")
                    || name.starts_with("java-")
                    || name.starts_with("jdk")
                    || name.starts_with("jre")
                    || name.starts_with("default-jdk")
                    || name.starts_with("default-jre"))
        }
        "dotnet" => name == "dotnet" || name.starts_with("dotnet-"),
        "ruby" => {
            matches!(name, "ruby" | "ruby-full")
                || name.strip_prefix("ruby").is_some_and(numeric_version)
        }
        "php" => {
            matches!(name, "php" | "php-cli")
                || name.strip_prefix("php").is_some_and(|rest| {
                    let rest = rest.strip_suffix("-cli").unwrap_or(rest);
                    numeric_version(rest)
                })
        }
        "zig" => name == "zig" || name.strip_prefix("zig").is_some_and(numeric_version),
        "bun" => name == "bun",
        "deno" => name == "deno",

        // Package managers
        "pnpm" => name == "pnpm",
        "yarn" => matches!(name, "yarn" | "yarnpkg"),
        "uv" => matches!(name, "uv" | "python-uv" | "python3-uv"),
        "pipx" => matches!(name, "pipx" | "python-pipx" | "python3-pipx"),
        "poetry" => matches!(name, "poetry" | "python-poetry" | "python3-poetry"),
        "composer" => name == "composer",

        // Version control
        "git" => name == "git",
        "git-lfs" => matches!(name, "git-lfs" | "gitlfs"),
        "gh" => matches!(name, "gh" | "github-cli"),
        "glab" => name == "glab",
        "lazygit" => name == "lazygit",
        "delta" => name == "git-delta",

        // Build & compilers
        "cmake" => name == "cmake",
        "gcc" => {
            name == "gcc"
                || name.strip_prefix("gcc-").is_some_and(|rest| {
                    !rest.is_empty()
                        && (numeric_version(rest)
                            || (rest.starts_with(|ch: char| ch.is_ascii_digit())
                                && rest.ends_with("-linux-gnu")))
                })
        }
        "clang" => name == "clang" || name.strip_prefix("clang-").is_some_and(numeric_version),
        "make" => matches!(name, "make" | "make-guile"),
        "ninja" => matches!(name, "ninja" | "ninja-build"),
        "meson" => name == "meson",
        "maven" => name == "maven",
        "gdb" => name == "gdb",
        "just" => name == "just",
        "protoc" => matches!(name, "protobuf-compiler" | "protobuf"),

        // Containers & cloud native
        "docker" => matches!(
            name,
            "docker.io" | "docker-ce-cli" | "docker-cli" | "docker" | "moby-engine"
        ),
        "podman" => name == "podman",
        "kubectl" => matches!(name, "kubectl" | "kubernetes-client"),
        "helm" => name == "helm",
        "terraform" => name == "terraform",
        "ansible" => matches!(name, "ansible" | "ansible-core"),

        // Database clients
        "sqlite" => matches!(name, "sqlite3" | "sqlite"),
        "psql" => {
            matches!(
                name,
                "postgresql-client" | "postgresql-client-common" | "postgresql"
            ) || name
                .strip_prefix("postgresql-client-")
                .is_some_and(numeric_version)
        }
        "mysql" => {
            matches!(
                name,
                "default-mysql-client"
                    | "mysql-client"
                    | "mysql-client-core"
                    | "mariadb-client"
                    | "mariadb-client-core"
                    | "mariadb"
                    | "mariadb-clients"
            ) || name
                .strip_prefix("mysql-client-")
                .or_else(|| name.strip_prefix("mariadb-client-"))
                .is_some_and(|rest| {
                    let rest = rest.strip_prefix("core-").unwrap_or(rest);
                    numeric_version(rest)
                })
        }
        "redis-cli" => matches!(
            name,
            "redis-tools" | "redis" | "valkey" | "valkey-compat-redis"
        ),

        // Editors & terminal CLI
        "neovim" => name == "neovim",
        "vim" => matches!(
            name,
            "vim"
                | "vim-tiny"
                | "vim-nox"
                | "vim-gtk3"
                | "vim-athena"
                | "vim-enhanced"
                | "vim-minimal"
        ),
        "tmux" => name == "tmux",
        "ripgrep" => name == "ripgrep",
        "fd" => matches!(name, "fd" | "fd-find"),
        "bat" => name == "bat",
        "jq" => name == "jq",
        "fzf" => name == "fzf",
        "curl" => name == "curl",
        "httpie" => name == "httpie",
        "shellcheck" => matches!(name, "shellcheck" | "ShellCheck"),
        "shfmt" => name == "shfmt",
        "direnv" => name == "direnv",
        "starship" => name == "starship",
        "zoxide" => name == "zoxide",
        "eza" => name == "eza",
        _ => false,
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_proc_has_any(names: &[&str]) -> Result<bool> {
    let entries = std::fs::read_dir("/proc").map_err(|error| Error::internal(error.to_string()))?;
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .chars()
            .all(|ch| ch.is_ascii_digit())
        {
            continue;
        }
        let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) else {
            continue;
        };
        if names.contains(&comm.trim()) {
            return Ok(true);
        }
    }
    Ok(false)
}
