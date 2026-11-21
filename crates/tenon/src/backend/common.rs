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
    match tool_id {
        "pnpm" => name == "pnpm",
        "git" => name == "git",
        "git-lfs" => matches!(name, "git-lfs" | "gitlfs"),
        "gh" => matches!(name, "gh" | "github-cli"),
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
        "cmake" => name == "cmake",
        "gcc" => {
            name == "gcc"
                || name.strip_prefix("gcc-").is_some_and(|rest| {
                    !rest.is_empty()
                        && (rest.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
                            || (rest.starts_with(|ch: char| ch.is_ascii_digit())
                                && rest.ends_with("-linux-gnu")))
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
        "python" => {
            let name = name.strip_suffix("-minimal").unwrap_or(name);
            name == "python"
                || name.strip_prefix("python").is_some_and(|rest| {
                    let rest = rest.trim_start_matches('@');
                    !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
                })
        }
        "rust" => matches!(name, "rust" | "rustc" | "cargo" | "rustup"),
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
