use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Executable, ExecutableSource, Identity, ToolSpec};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Compare command directories using Windows' case and extended-path conventions.
pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        let normalize = |path: &Path| {
            let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
            if let Some(unc) = text.strip_prefix(r"\\?\unc\") {
                format!(r"\\{unc}")
            } else {
                text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
            }
        };
        normalize(left) == normalize(right)
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

pub(crate) fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    metadata.is_file()
}

pub(crate) fn search_directories() -> Vec<PathBuf> {
    let mut directories: Vec<_> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|path| path.is_absolute())
                .collect()
        })
        .unwrap_or_default();
    if cfg!(unix) {
        for path in ["/usr/bin", "/usr/local/bin", "/bin"] {
            directories.push(path.into());
        }
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        for suffix in [".local/bin", ".cargo/bin"] {
            directories.push(home.join(suffix));
        }
    }
    let mut seen = BTreeSet::new();
    directories.retain(|path| seen.insert(path.clone()));
    directories
}

pub(crate) fn find(program: &str) -> Option<PathBuf> {
    let paths = std::env::join_paths(search_directories()).ok()?;
    which::which_in(program, Some(paths), std::env::temp_dir()).ok()
}

pub(crate) fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn active_on_path(spec: &ToolSpec) -> Vec<(String, PathBuf)> {
    spec.programs
        .iter()
        .filter_map(|program| {
            which::which_in(program, std::env::var_os("PATH"), std::env::temp_dir())
                .ok()
                .map(|path| ((*program).into(), path))
        })
        .collect()
}

pub(crate) async fn probe_active(spec: &ToolSpec) -> Vec<Executable> {
    let active = active_on_path(spec);
    let mut results = Vec::new();
    for (program, path) in &active {
        if let Some(entry) = inspect_entry(spec, path, program.clone(), vec![program.clone()]).await
        {
            results.push(entry);
        }
    }
    results
}

fn match_program_name(spec: &ToolSpec, path: &Path) -> Option<String> {
    let raw = path.file_name()?.to_str()?;
    #[cfg(windows)]
    let normalized = {
        let lower = raw.to_ascii_lowercase();
        lower
            .strip_suffix(".exe")
            .or_else(|| lower.strip_suffix(".cmd"))?
            .to_owned()
    };
    #[cfg(not(windows))]
    let normalized = raw.to_owned();

    let name = normalized.as_str();
    if spec.programs.contains(&name) {
        return Some(normalized);
    }
    if spec.versioned
        && spec.programs.iter().any(|program| {
            name.strip_prefix(program).is_some_and(|suffix| {
                let suffix = suffix.trim_start_matches(['-', '.']);
                suffix.starts_with(|ch: char| ch.is_ascii_digit())
                    && suffix.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
            })
        })
    {
        return Some(normalized);
    }
    None
}

async fn inspect_entry(
    spec: &ToolSpec,
    path: &Path,
    program: String,
    active_programs: Vec<String>,
) -> Option<Executable> {
    let (version, issue) = match process::version(path, spec.version_arguments).await {
        Ok(version) => (Some(version), None),
        Err(error) => (None, Some(error.message)),
    };
    // Disabled Windows Store aliases are OS placeholders, not real installations.
    if version.is_none() && is_os_placeholder(spec.id, path) {
        return None;
    }
    Some(Executable {
        path: path.to_string_lossy().into(),
        resolved_path: resolved(path).to_string_lossy().into(),
        program,
        version,
        on_path: !active_programs.is_empty(),
        active_programs,
        source: ExecutableSource::External,
        issue,
    })
}

fn is_os_placeholder(tool_id: &str, path: &Path) -> bool {
    cfg!(windows)
        && tool_id == "python"
        && path
            .parent()
            .is_some_and(|parent| parent.file_name().is_some_and(|name| name == "WindowsApps"))
        && path.metadata().is_ok_and(|metadata| metadata.len() == 0)
}

pub(crate) async fn scan(spec: &ToolSpec, package_files: &[PathBuf]) -> Result<Vec<Executable>> {
    let path_directories: Vec<_> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    let active = active_on_path(spec);
    let mut candidates: Vec<_> = active.iter().map(|(_, path)| path.clone()).collect();
    for directory in search_directories() {
        match std::fs::read_dir(&directory) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(|error| {
                        Error::internal(format!("无法完整读取 {}：{error}", directory.display()))
                    })?;
                    let path = entry.path();
                    if match_program_name(spec, &path).is_some() {
                        candidates.push(path);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::internal(format!(
                    "无法检查 {}，为避免重复安装已停止检测：{error}",
                    directory.display()
                )));
            }
        }
    }
    candidates.extend_from_slice(package_files);
    candidates.retain(|path| {
        match_program_name(spec, path).is_some()
            || active.iter().any(|(_, selected)| selected == path)
    });
    // Keep the PATH entry first when aliases and package records refer to one binary.
    candidates.sort_by_key(|path| {
        let order = path
            .parent()
            .and_then(|parent| path_directories.iter().position(|dir| dir == parent))
            .unwrap_or(usize::MAX);
        (
            !active.iter().any(|(_, selected)| selected == path),
            order,
            path.clone(),
        )
    });

    let mut seen: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut results: Vec<Executable> = Vec::new();
    for path in candidates {
        let Some(program) = active
            .iter()
            .find(|(_, selected)| selected == &path)
            .map(|(program, _)| program.clone())
            .or_else(|| match_program_name(spec, &path))
        else {
            continue;
        };
        if !is_executable(&path) {
            continue;
        }
        let active_programs: Vec<_> = active
            .iter()
            .filter(|(_, selected)| same_path(selected, &path))
            .map(|(program, _)| program.clone())
            .collect();
        let canonical = resolved(&path);
        let component = if spec.identity == Identity::PerProgram {
            program.clone()
        } else {
            spec.id.into()
        };
        let identity = (component, file_identity(&path, &canonical));
        if let Some(index) = seen.get(&identity).copied() {
            let entry = &mut results[index];
            for program in active_programs {
                if !entry.active_programs.contains(&program) {
                    entry.active_programs.push(program);
                }
            }
            entry.on_path = !entry.active_programs.is_empty();
            continue;
        }
        if seen.len() >= 64 {
            return Err(Error::conflict(format!(
                "{} 的安装入口超过 64 个，无法完整读取；未开放环境修改。",
                spec.name
            )));
        }
        if let Some(entry) = inspect_entry(spec, &path, program, active_programs).await {
            seen.insert(identity, results.len());
            results.push(entry);
        }
    }
    Ok(results)
}

fn file_identity(path: &Path, canonical: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(metadata) = path.metadata() {
            return format!("{}:{}", metadata.dev(), metadata.ino());
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    canonical.to_string_lossy().into()
}
