use super::WingetBackend;
use crate::backend::{CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Action, Change, Preview, PreviewState};

pub(super) async fn prepare(
    backend: &WingetBackend,
    action: Action,
    names: &[String],
) -> Result<Preview> {
    let mut targets = Vec::new();
    let mut changes = Vec::new();
    let mut details = String::new();

    for name in names {
        let mut candidate = None;
        if action == Action::Update {
            let output = process::run(
                &backend.path,
                &process::args(&[
                    "list",
                    "--id",
                    name,
                    "--exact",
                    "--source",
                    "winget",
                    "--upgrade-available",
                    "--accept-source-agreements",
                    "--disable-interactivity",
                ]),
                false,
            )
            .await?;
            let no_upgrade = output.status.code().map(|code| code as u32) == Some(0x8A150014);
            if !output.status.success() && !no_upgrade {
                return Err(output.checked().unwrap_err());
            }
            for line in output.stdout.lines() {
                let words: Vec<_> = line.split_whitespace().collect();
                if let Some(index) = words.iter().position(|word| *word == name) {
                    candidate = words
                        .get(index + 2)
                        .filter(|word| **word != "winget")
                        .map(|word| (*word).to_owned());
                    if candidate.is_none() {
                        return Err(Error::internal("WinGet 未返回明确的升级版本。"));
                    }
                }
            }
            if candidate.is_none() && !no_upgrade {
                return Err(Error::internal(
                    "WinGet 没有返回可识别的更新状态，请重新检查。",
                ));
            }
        } else {
            let versions = process::run(
                &backend.path,
                &process::args(&[
                    "show",
                    "--id",
                    name,
                    "--exact",
                    "--source",
                    "winget",
                    "--versions",
                    "--accept-source-agreements",
                    "--disable-interactivity",
                ]),
                false,
            )
            .await?
            .checked_stdout()?;
            let mut table = false;
            for line in versions.lines() {
                if line.trim().len() >= 3 && line.trim().chars().all(|ch| ch == '-') {
                    table = true;
                    continue;
                }
                if table && !line.trim().is_empty() {
                    candidate = Some(line.trim().into());
                    break;
                }
            }
            if candidate.is_none() {
                return Err(Error::not_found(format!(
                    "WinGet 没有返回 {name} 的可安装版本。"
                )));
            }
        }

        let installed = backend.installed_version_for(name);
        let target = Change {
            name: name.clone(),
            installed: installed.clone(),
            candidate: candidate.clone().or(installed),
            dependency: false,
        };
        if let Some(version) = candidate {
            let info = process::run(
                &backend.path,
                &process::args(&[
                    "show",
                    "--id",
                    name,
                    "--exact",
                    "--source",
                    "winget",
                    "--version",
                    &version,
                    "--accept-source-agreements",
                    "--disable-interactivity",
                ]),
                false,
            )
            .await?
            .checked_stdout()?;
            details.push_str(&info);
            details.push('\n');
            changes.push(target.clone());
        }
        targets.push(target);
    }

    let basis = match &backend.catalog_basis {
        Some(basis) => format!("{basis}；安装器声明依赖"),
        None => "WinGet 安装器及声明依赖".into(),
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

pub(super) async fn check_updates(
    backend: &WingetBackend,
    names: &[String],
) -> Result<CandidateTargets> {
    let output = process::run(
        &backend.path,
        &process::args(&[
            "list",
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]),
        false,
    )
    .await?
    .checked_stdout()?;

    Ok(names
        .iter()
        .map(|name| {
            let single = [name.as_str()];
            let candidates: &[&str] = if name == "OpenJS.NodeJS" {
                &["OpenJS.NodeJS.LTS", "OpenJS.NodeJS"]
            } else {
                &single
            };
            let rows: Vec<_> = output
                .lines()
                .filter_map(|line| {
                    let words: Vec<_> = line.split_whitespace().collect();
                    candidates.iter().find_map(|candidate| {
                        words
                            .iter()
                            .position(|word| word == candidate)
                            .map(|at| ((*candidate).to_owned(), words[at + 1..].to_vec()))
                    })
                })
                .collect();
            let target = match rows.as_slice() {
                [(matched_id, row)]
                    if matches!(row.len(), 2 | 3)
                        && row.last() == Some(&"winget")
                        && row[..row.len() - 1]
                            .iter()
                            .all(|field| field.chars().any(|ch| ch.is_ascii_digit()))
                        && !row.iter().any(|field| field.contains('…')) =>
                {
                    Ok(Change {
                        name: matched_id.clone(),
                        installed: Some(row[0].into()),
                        candidate: Some(row[if row.len() == 3 { 1 } else { 0 }].into()),
                        dependency: false,
                    })
                }
                _ => Err(Error::internal(format!(
                    "WinGet 没有返回 {name} 的明确更新状态，请刷新后重试。"
                ))),
            };
            (name.clone(), target)
        })
        .collect())
}
