use super::{NpmBackend, mapping, metadata};
use crate::backend::{BackendContext, common};
use crate::error::{Error, Result};
use crate::infra::{paths, privilege, process};
use crate::model::{
    Action, BackendId, Change, Command, Plan, Preview, PreviewState, ToolReport, ToolSpec,
};

pub(super) async fn prepare(
    ctx: &BackendContext,
    backend: &NpmBackend,
    spec: &ToolSpec,
    tool: ToolReport,
    action: Action,
    selected: Option<String>,
) -> Result<Plan> {
    let npm = &backend.installation;
    let name = mapping::package_name(spec.id)
        .ok_or_else(|| Error::unsupported("此工具没有 npm 软件包。"))?;
    let current = common::installed_version(&backend.installed, name);
    let candidate = if action == Action::Uninstall {
        None
    } else if let Some(selected) = &selected {
        let (package, version) = selected
            .split_once('=')
            .ok_or_else(|| Error::invalid_input("所选 npm 版本无效。"))?;
        if package != name || metadata::semver_triplet(version).is_none() {
            return Err(Error::invalid_input("所选 npm 版本无效。"));
        }
        Some(version.to_owned())
    } else {
        let latest = metadata::latest(ctx, backend, name).await?;
        Some(
            if action == Action::Update
                && current
                    .as_deref()
                    .and_then(metadata::semver_triplet)
                    .zip(metadata::semver_triplet(&latest))
                    .is_some_and(|(current, latest)| current > latest)
            {
                current
                    .clone()
                    .ok_or_else(|| Error::plan_expired("npm 安装记录缺失。"))?
            } else {
                latest
            },
        )
    };

    let unchanged = current == candidate;
    let administrator = !privilege::is_writable(&npm.prefix)
        || !privilege::is_writable(&npm.root)
        || !privilege::is_writable(&npm.bin_directory());
    if !unchanged && administrator && cfg!(windows) {
        return Err(Error::authorization(
            "npm 全局目录需要管理员权限，请先通过 npm 配置可写的全局目录。",
        ));
    }
    if !unchanged && administrator && !privilege::has_pkexec() {
        return Err(Error::authorization(
            "npm 全局目录需要系统授权，但当前系统没有可用的授权组件。",
        ));
    }
    if candidate.is_some()
        && !std::env::var_os("PATH").is_some_and(|path| {
            std::env::split_paths(&path).any(|entry| paths::same_path(&entry, &npm.bin_directory()))
        })
    {
        return Err(Error::conflict(format!(
            "npm 命令目录 {} 不在当前进程的 PATH 中。",
            npm.bin_directory().display()
        )));
    }

    let mut args = vec![
        if candidate.is_some() {
            "install"
        } else {
            "uninstall"
        }
        .into(),
        "--global".into(),
    ];
    args.push(
        candidate
            .as_ref()
            .map_or_else(|| name.to_owned(), |version| format!("{name}@{version}")),
    );
    args.extend(process::args(&["--engine-strict", "--foreground-scripts"]));
    if candidate.is_some() {
        // Targets are exact versions. Reuse verified npm cache entries while still
        // fetching missing data and enforcing engines during the dry run/install.
        args.push("--prefer-offline".into());
    }

    if !unchanged && candidate.is_some() {
        let mut preview_args = args.clone();
        preview_args.extend(process::args(&["--dry-run", "--ignore-scripts", "--json"]));
        npm.dry_run_json(&preview_args).await.map_err(|error| {
            Error::conflict(format!(
                "所选版本的安装条件未通过，请检查 Node.js 版本及平台支持。\n{error}"
            ))
        })?;
    }

    let targets = vec![Change {
        name: name.into(),
        installed: current,
        candidate: candidate.clone(),
        dependency: false,
    }];
    let node_version = process::version(&npm.node, &["--version"]).await?;
    let basis = format!(
        "npm 全局目录：{}；软件源：{}；Node.js {}",
        npm.prefix.display(),
        npm.registry,
        node_version
    );
    let commands = if unchanged {
        Vec::new()
    } else {
        vec![Command {
            program: npm.node.to_string_lossy().into_owned(),
            arguments: npm.arguments(&args),
            administrator,
        }]
    };

    Ok(Plan {
        tool,
        action,
        backend: BackendId::Npm,
        packages: vec![name.into()],
        commands,
        impact: "修改 npm 全局安装。".into(),
        preview: Preview {
            state: if unchanged {
                PreviewState::Current
            } else {
                PreviewState::Available
            },
            targets: targets.clone(),
            changes: targets,
            details: String::new(),
            basis,
        },
        version: candidate.map(|version| format!("{name}={version}")),
    })
}
