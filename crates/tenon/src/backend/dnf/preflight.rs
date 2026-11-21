use super::DnfBackend;
use crate::backend::{CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Action, Change, Preview, PreviewState};

pub(super) async fn prepare(
    backend: &DnfBackend,
    action: Action,
    names: &[String],
) -> Result<Preview> {
    let output = backend
        .query(
            &[
                "--cacheonly",
                "repoquery",
                "--available",
                "--latest-limit",
                "1",
                "--qf",
                "%{name}\t%{evr}\t%{arch}",
            ],
            names,
        )
        .await?;

    let mut targets = Vec::new();
    for name in names {
        let records: Vec<_> = output
            .lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .filter(|fields| fields.len() == 3 && fields[0] == name)
            .collect();
        let record = records.first().ok_or_else(|| {
            Error::not_found(format!(
                "DNF 缓存没有 {name} 的可安装版本。请刷新软件源后重试。"
            ))
        })?;
        if records.iter().any(|other| other[1] != record[1]) {
            return Err(Error::conflict(format!(
                "{name} 有多个架构的不同候选版本，请通过 DNF 审阅。"
            )));
        }
        targets.push(Change {
            name: name.clone(),
            installed: common::installed_version(&backend.installed, name),
            candidate: Some(record[1].strip_prefix("0:").unwrap_or(record[1]).into()),
            dependency: false,
        });
    }

    if action == Action::Update {
        let mut args = backend.catalog_arguments.clone();
        args.extend(process::args(&["--cacheonly", "check-update"]));
        args.extend_from_slice(names);
        let result = process::run(&backend.path, &args, false).await?;
        if result.status.success() {
            for target in &mut targets {
                target.candidate = target.installed.clone();
            }
            return Ok(Preview {
                state: PreviewState::Current,
                basis: backend
                    .catalog_basis
                    .clone()
                    .unwrap_or_else(|| "本机 DNF 缓存".into()),
                targets,
                changes: Vec::new(),
                details: String::new(),
            });
        }
        if result.status.code() != Some(100) {
            return Err(result.checked().unwrap_err());
        }
    }

    let dependencies = backend
        .query(
            &[
                "--cacheonly",
                "repoquery",
                "--requires",
                "--resolve",
                "--recursive",
                "--qf",
                "%{name}\t%{evr}\t%{arch}",
            ],
            names,
        )
        .await?;
    let mut details = String::from("依赖候选（包名 / 版本 / 架构）：\n");
    details.push_str(&dependencies);

    let basis = match &backend.catalog_basis {
        Some(basis) => format!("{basis}；依赖候选，实际事务在执行时解析"),
        None => "DNF 依赖候选；实际事务在执行时解析".into(),
    };
    let preview = Preview {
        state: PreviewState::Available,
        basis,
        changes: targets.clone(),
        targets,
        details,
    };
    common::validate_preview_limits(&preview)?;
    Ok(preview)
}

pub(super) async fn check_updates(
    backend: &DnfBackend,
    names: &[String],
) -> Result<CandidateTargets> {
    let output = backend
        .query(
            &[
                "--cacheonly",
                "--quiet",
                "repoquery",
                "--upgrades",
                "--latest-limit",
                "1",
                "--qf",
                "%{name}\t%{evr}\t%{arch}",
            ],
            names,
        )
        .await?;
    let mut targets = common::unchanged_targets(&backend.installed, names);
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
            return Err(Error::internal("无法完整解析 DNF 更新列表。"));
        }
        let Some(target) = targets.get_mut(fields[0]) else {
            continue;
        };
        let Ok(target) = target else {
            continue;
        };
        let candidate = fields[1].strip_prefix("0:").unwrap_or(fields[1]);
        if target.candidate != target.installed && target.candidate.as_deref() != Some(candidate) {
            targets.insert(
                fields[0].into(),
                Err(Error::conflict("多个架构的更新版本不同，请通过 DNF 审阅。")),
            );
        } else {
            target.candidate = Some(candidate.into());
        }
    }
    Ok(targets)
}
