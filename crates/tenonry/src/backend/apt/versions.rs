use super::{AptBackend, activation, mapping, preflight};
use crate::backend::BackendDriver;
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{
    Action, BackendId, Change, Command, Package, Plan, PreviewState, ToolReport, ToolSpec,
    VersionInfo, VersionList,
};
use crate::registry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn version_key(package: &Package) -> String {
    format!("{}={}", package.name, package.version)
}

async fn query_cache(backend: &AptBackend, args: &[String]) -> Result<String> {
    let mut arguments = backend.catalog_arguments.clone();
    arguments.extend_from_slice(args);
    process::run(Path::new("/usr/bin/apt-cache"), &arguments, false)
        .await?
        .checked_stdout()
}

fn parse_records(output: &str) -> Vec<BTreeMap<&str, &str>> {
    output
        .split("\n\n")
        .filter(|text| !text.trim().is_empty())
        .map(|text| {
            text.lines()
                .filter_map(|line| line.split_once(": "))
                .filter(|(key, _)| !key.starts_with(char::is_whitespace))
                .collect()
        })
        .collect()
}

pub(super) async fn available_packages(
    backend: &AptBackend,
    spec: &ToolSpec,
) -> Result<Vec<Package>> {
    let mut names = Vec::new();
    for prefix in mapping::version_query_prefixes(spec.id) {
        let output = query_cache(backend, &process::args(&["pkgnames", prefix])).await?;
        names.extend(
            output
                .lines()
                .filter(|name| mapping::is_primary_package(spec.id, name))
                .map(str::to_owned),
        );
    }
    names.sort();
    names.dedup();
    if names.is_empty() {
        return Ok(Vec::new());
    }
    if names.len() > 128 {
        return Err(Error::conflict("软件源返回的工具系列过多，无法完整读取。"));
    }

    let mut args = vec!["show".into()];
    args.extend(names);
    let output = query_cache(backend, &args).await?;
    let arch = preflight::native_architecture().await?;

    let mut packages = Vec::new();
    for record in parse_records(&output) {
        let name = record
            .get("Package")
            .ok_or_else(|| Error::internal("软件源缺少包名。"))?;
        let version = record
            .get("Version")
            .ok_or_else(|| Error::internal("软件源缺少版本。"))?;
        if !mapping::is_primary_package(spec.id, name) || !record.contains_key("Filename") {
            continue;
        }
        if !record
            .get("Architecture")
            .is_some_and(|value| *value == arch || *value == "all")
        {
            continue;
        }
        let upstream = version.split('-').next().unwrap_or(version);
        if ["ea", "alpha", "beta", "rc"]
            .iter()
            .any(|part| upstream.contains(part))
        {
            continue;
        }
        if !version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ".+:~-".contains(ch))
        {
            return Err(Error::internal("软件源返回了无法识别的版本。"));
        }
        let package = Package {
            name: (*name).into(),
            version: (*version).into(),
        };
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    Ok(packages)
}

pub(super) async fn list_versions(
    backend: &AptBackend,
    spec: &ToolSpec,
    tool: &ToolReport,
) -> Result<VersionList> {
    let candidates = available_packages(backend, spec).await?;
    let active = tool
        .status
        .version
        .as_deref()
        .and_then(registry::extract_release);
    let mut versions: Vec<_> = candidates
        .into_iter()
        .map(|package| VersionInfo {
            version: version_key(&package),
            label: package.version.clone(),
            current: tool.packages.contains(&package)
                && active.as_deref().is_some_and(|active| {
                    registry::extract_release(&package.version)
                        .is_some_and(|release| registry::releases_match(&release, active))
                }),
        })
        .collect();
    if !versions.iter().any(|version| version.current)
        && let Some(label) = tool.status.version.clone()
    {
        versions.push(VersionInfo {
            version: "current".into(),
            label,
            current: true,
        });
    }
    Ok(VersionList {
        tool_id: spec.id.into(),
        backend: Some(BackendId::Apt),
        versions,
        message: String::new(),
    })
}

pub(super) async fn plan_version(
    backend: &AptBackend,
    spec: &ToolSpec,
    tool: ToolReport,
    action: Action,
    version: String,
) -> Result<Plan> {
    let candidate = available_packages(backend, spec)
        .await?
        .into_iter()
        .find(|package| version_key(package) == version)
        .ok_or_else(|| Error::plan_expired("所选版本已不在软件源中，请重新查询可用版本。"))?;

    let selected = mapping::package_series(spec.id, &candidate.name);
    let mut install = BTreeSet::from([candidate.name.clone()]);
    if spec.id == "java" {
        for suffix in ["-jre", "-jre-headless", "-jdk-headless"] {
            install.insert(format!("{selected}{suffix}"));
        }
    }
    if spec.id == "go" {
        install.insert(format!("{selected}-src"));
    }
    if spec.id == "rust" {
        install.insert("cargo".into());
    }
    if spec.id == "python" {
        install.extend([
            format!("{selected}-minimal"),
            format!("lib{selected}-stdlib"),
            format!("lib{selected}-minimal"),
        ]);
    }

    let mut targets: Vec<_> = install
        .iter()
        .map(|name| Change {
            name: name.clone(),
            installed: installed_by_base_name(backend, name),
            candidate: Some(candidate.version.clone()),
            dependency: false,
        })
        .collect();

    if spec.id == "node" {
        let (mut candidates, _) = preflight::candidates(backend, &["npm".into()]).await?;
        targets.push(
            candidates
                .remove("npm")
                .ok_or_else(|| Error::not_found("未找到 npm 安装信息。"))??,
        );
        install.insert("npm".into());
    }

    for package in &tool.packages {
        let name = mapping::base_name(&package.name);
        if !mapping::belongs_to_series(spec.id, name, selected) {
            targets.push(Change {
                name: package.name.clone(),
                installed: Some(package.version.clone()),
                candidate: None,
                dependency: false,
            });
        }
    }

    if spec.id == "go" {
        for package in &backend.installed {
            if package.name.starts_with("golang-")
                && package.name.ends_with("-src")
                && !mapping::belongs_to_series(spec.id, &package.name, selected)
            {
                targets.push(Change {
                    name: package.name.clone(),
                    installed: Some(package.version.clone()),
                    candidate: None,
                    dependency: false,
                });
            }
        }
    }

    if targets
        .iter()
        .any(|target| backend.is_protected_package(&target.name))
    {
        return Err(Error::protected("系统自带组件不参与版本替换。"));
    }

    let preview = preflight::simulate_replacement(backend, action, targets).await?;
    if let Some(issue) = backend.authorization_issue() {
        return Err(Error::authorization(issue));
    }
    let mut commands = Vec::new();
    if preview.state != PreviewState::Current {
        let mut args = backend.execution_catalog_arguments();
        args.extend(process::args(&[
            "--yes",
            "--allow-downgrades",
            "--no-install-recommends",
            "-o",
            "Dpkg::Options::=--force-confdef",
            "-o",
            "Dpkg::Options::=--force-confold",
            "install",
        ]));
        args.extend(preflight::pinned_arguments(&preview.targets));
        // Download first so network failures stop before any old package is removed.
        for flag in ["--download-only", "--no-download"] {
            let mut arguments = args.clone();
            arguments.insert(0, flag.into());
            commands.push(Command {
                program: backend.path.to_string_lossy().into(),
                arguments,
                administrator: true,
            });
        }
        commands.extend(activation::install_commands(spec.id, &candidate.name)?);
    }

    Ok(Plan {
        tool,
        action,
        backend: BackendId::Apt,
        packages: install.into_iter().collect(),
        commands,
        impact: "替换当前版本。".into(),
        preview,
        version: Some(version),
    })
}

pub(super) async fn newest_primary_package(
    backend: &AptBackend,
    spec: &ToolSpec,
) -> Result<Option<String>> {
    let candidates = available_packages(backend, spec).await?;
    let best = candidates
        .into_iter()
        .filter_map(|pkg| {
            let release = registry::extract_release(&pkg.version)?;
            let parts: Vec<u64> = release
                .split('.')
                .map(|part| part.parse::<u64>().ok())
                .collect::<Option<Vec<_>>>()?;
            Some((parts, pkg.name))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, name)| name);
    Ok(best)
}

fn installed_by_base_name(backend: &AptBackend, name: &str) -> Option<String> {
    backend
        .installed
        .iter()
        .find(|package| mapping::base_name(&package.name) == name)
        .map(|package| package.version.clone())
}
