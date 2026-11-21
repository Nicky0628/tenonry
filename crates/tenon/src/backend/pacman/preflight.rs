use super::PacmanBackend;
use crate::backend::{CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Action, Change, Preview, PreviewState};

pub(super) async fn prepare(
    backend: &PacmanBackend,
    action: Action,
    names: &[String],
) -> Result<Preview> {
    let mut args = process::args(&["-Sp", "--needed", "--print-format", "%n\t%v\t%r"]);
    args.extend_from_slice(names);
    let output = process::run(&backend.path, &args, false)
        .await?
        .checked_stdout()?;

    let mut changes = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(Error::internal(
                "无法解析 pacman 的事务预览，请先检查同步数据库。",
            ));
        }
        changes.push(Change {
            name: fields[0].into(),
            installed: common::installed_version(&backend.installed, fields[0]),
            candidate: Some(fields[1].into()),
            dependency: !names.iter().any(|name| name == fields[0]),
        });
    }

    let mut targets = Vec::new();
    for name in names {
        if let Some(change) = changes.iter().find(|change| change.name == *name) {
            targets.push(change.clone());
        } else if let Some(version) = common::installed_version(&backend.installed, name) {
            targets.push(Change {
                name: name.clone(),
                installed: Some(version.clone()),
                candidate: Some(version),
                dependency: false,
            });
        } else {
            return Err(Error::not_found(format!(
                "pacman 没有返回 {name} 的可安装版本。"
            )));
        }
    }

    let state = if action == Action::Update && changes.is_empty() {
        PreviewState::Current
    } else {
        PreviewState::Available
    };
    let preview = Preview {
        state,
        basis: backend
            .catalog_basis
            .clone()
            .unwrap_or_else(|| "本机 pacman 同步数据库".into()),
        targets,
        changes,
        details: output,
    };
    common::validate_preview_limits(&preview)?;
    Ok(preview)
}

pub(super) async fn check_updates(
    backend: &PacmanBackend,
    names: &[String],
) -> Result<CandidateTargets> {
    let output = process::run(&backend.path, &process::args(&["-Qu"]), false).await?;
    let empty = output.status.code() == Some(1) && output.text.trim().is_empty();
    if !output.status.success() && !empty {
        return Err(output.checked().unwrap_err());
    }
    let mut targets = common::unchanged_targets(&backend.installed, names);
    for line in output.stdout.lines().filter(|line| !line.trim().is_empty()) {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() < 4 || words[2] != "->" {
            return Err(Error::internal("无法完整解析 pacman 更新列表。"));
        }
        if let Some(Ok(target)) = targets.get_mut(words[0]) {
            target.candidate = Some(words[3].into());
        }
    }
    Ok(targets)
}
