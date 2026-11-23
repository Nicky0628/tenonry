use super::AptBackend;
use crate::backend::{CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Action, Change, Preview, PreviewState};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) async fn prepare(
    backend: &AptBackend,
    action: Action,
    names: &[String],
) -> Result<Preview> {
    let (candidates, native_suffix) = candidates(backend, names).await?;
    let targets = names
        .iter()
        .map(|name| {
            candidates
                .get(name)
                .cloned()
                .ok_or_else(|| Error::internal("APT 候选记录缺失。"))?
        })
        .collect::<Result<Vec<_>>>()?;
    simulate_transaction(backend, action, targets, &native_suffix, false).await
}

pub(super) async fn simulate_replacement(
    backend: &AptBackend,
    action: Action,
    targets: Vec<Change>,
) -> Result<Preview> {
    let architecture = native_architecture().await?;
    simulate_transaction(backend, action, targets, &format!(":{architecture}"), true).await
}

pub(super) fn pinned_arguments(targets: &[Change]) -> Vec<String> {
    targets
        .iter()
        .map(|target| match &target.candidate {
            Some(version) => format!("{}={version}", target.name),
            None => format!("{}-", target.name),
        })
        .collect()
}

pub(super) async fn check_updates(
    backend: &AptBackend,
    names: &[String],
) -> Result<CandidateTargets> {
    let (mut targets, _) = candidates(backend, names).await?;
    for value in targets.values_mut() {
        let Ok(target) = value else {
            continue;
        };
        let (Some(current), Some(candidate)) = (&target.installed, &target.candidate) else {
            *value = Err(Error::plan_expired("安装状态已变化，请刷新后重试。"));
            continue;
        };
        if current == candidate {
            continue;
        }
        let comparison = process::run(
            Path::new("/usr/bin/dpkg"),
            &process::args(&["--compare-versions", candidate, "gt", current]),
            false,
        )
        .await?;
        if !comparison.status.success() {
            *value = Err(if comparison.status.code() == Some(1) {
                Error::conflict("候选版本低于当前版本，请通过 APT 审阅版本限制。")
            } else {
                comparison.checked().unwrap_err()
            });
        }
    }
    Ok(targets)
}

async fn simulate_transaction(
    backend: &AptBackend,
    action: Action,
    targets: Vec<Change>,
    native_suffix: &str,
    selected: bool,
) -> Result<Preview> {
    let pinned = pinned_arguments(&targets);
    let mut flags = vec!["--simulate"];
    if selected {
        flags.extend(["--allow-downgrades", "--no-install-recommends"]);
    } else {
        flags.push("--no-remove");
    }
    if action == Action::Update && !selected {
        flags.push("--only-upgrade");
    }
    flags.push("install");

    let mut args = backend.catalog_arguments.clone();
    args.extend(process::args(&flags));
    args.extend(pinned);
    let details = process::run(&backend.path, &args, false)
        .await?
        .checked_stdout()?;

    if !selected && details.contains("will be DOWNGRADED") {
        return Err(Error::conflict(
            "候选版本低于当前版本，请指定目标版本后重试。",
        ));
    }

    let mut changes = Vec::new();
    for line in details.lines() {
        if let Some(rest) = line.strip_prefix("Remv ") {
            let name = rest
                .split_whitespace()
                .next()
                .ok_or_else(|| Error::internal("无法解析移除的软件包。"))?;
            let target = targets
                .iter()
                .find(|target| {
                    target.candidate.is_none() && same_package(&target.name, name, native_suffix)
                })
                .ok_or_else(|| {
                    Error::conflict(format!(
                        "替换会额外卸载 {name}，已停止。请先处理它对旧版本的依赖。"
                    ))
                })?;
            changes.push(target.clone());
        }
        if let Some(rest) = line.strip_prefix("Inst ") {
            let (name, rest) = rest
                .split_once(' ')
                .ok_or_else(|| Error::internal("无法解析 APT 依赖变化。"))?;
            let candidate = rest
                .split_once('(')
                .and_then(|(_, value)| value.split_whitespace().next())
                .ok_or_else(|| Error::internal("无法解析 APT 目标版本。"))?;
            let installed = rest
                .strip_prefix('[')
                .and_then(|value| value.split_once(']'))
                .map(|(value, _)| value.to_owned());
            changes.push(Change {
                name: name.into(),
                installed,
                candidate: Some(candidate.into()),
                dependency: !targets
                    .iter()
                    .any(|target| same_package(&target.name, name, native_suffix)),
            });
        }
    }

    if changes
        .iter()
        .any(|change| backend.is_protected_package(&change.name))
    {
        return Err(Error::protected(
            "此版本需要修改系统自带组件，请先通过系统更新处理。",
        ));
    }
    if targets.iter().any(|target| {
        target.installed != target.candidate
            && !changes.iter().any(|change| {
                same_package(&target.name, &change.name, native_suffix)
                    && change.candidate == target.candidate
            })
    }) {
        return Err(Error::conflict(
            "包管理器未能生成完整的替换计划，请检查依赖或保留包设置。",
        ));
    }

    let basis = match &backend.catalog_basis {
        Some(basis) if !selected => format!("{basis}；已锁定目标版本"),
        Some(basis) => basis.clone(),
        None => "本机 APT 索引".into(),
    };
    let preview = Preview {
        state: if changes.is_empty() {
            PreviewState::Current
        } else {
            PreviewState::Available
        },
        basis,
        targets,
        changes,
        details,
    };
    common::validate_preview_limits(&preview)?;
    Ok(preview)
}

pub(super) async fn candidates(
    backend: &AptBackend,
    names: &[String],
) -> Result<(CandidateTargets, String)> {
    let architecture = native_architecture().await?;
    let native_suffix = format!(":{architecture}");
    let mut arguments = backend.catalog_arguments.clone();
    arguments.push("policy".into());
    arguments.extend_from_slice(names);
    let policy = process::run(Path::new("/usr/bin/apt-cache"), &arguments, false)
        .await?
        .checked_stdout()?;

    let mut targets = BTreeMap::new();
    for name in names {
        let target = (|| {
            let mut inside = false;
            let mut installed = None;
            let mut candidate = None;
            for line in policy.lines() {
                if !line.starts_with(char::is_whitespace) {
                    inside = same_package(name, line.trim_end_matches(':'), &native_suffix);
                } else if inside {
                    if let Some(value) = line.trim().strip_prefix("Installed: ") {
                        installed = (value != "(none)").then(|| value.to_owned());
                    }
                    if let Some(value) = line.trim().strip_prefix("Candidate: ") {
                        candidate = (value != "(none)").then(|| value.to_owned());
                    }
                }
            }
            let candidate = candidate.ok_or_else(|| {
                Error::not_found(format!(
                    "当前 APT 索引没有 {name} 的可安装版本。请检查软件源或刷新索引后重试。"
                ))
            })?;
            Ok(Change {
                name: name.clone(),
                installed,
                candidate: Some(candidate),
                dependency: false,
            })
        })();
        targets.insert(name.clone(), target);
    }
    Ok((targets, native_suffix))
}

pub(super) async fn native_architecture() -> Result<String> {
    let output = process::run(
        Path::new("/usr/bin/dpkg"),
        &process::args(&["--print-architecture"]),
        false,
    )
    .await?
    .checked_stdout()?;
    Ok(output.trim().to_owned())
}

fn same_package(requested: &str, actual: &str, native_suffix: &str) -> bool {
    requested == actual
        || requested
            .strip_suffix(native_suffix)
            .is_some_and(|base| base == actual)
}
