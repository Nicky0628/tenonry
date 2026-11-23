use crate::backend::{Backend, BackendSet};
use crate::engine::Engine;
use crate::error::Result;
use crate::infra::{clock, paths, probe};
use crate::model::{
    Action, BackendId, ExecutableSource, Package, ProbeReport, SystemReport, ToolReport,
    ToolSource, ToolSpec, ToolState,
};
use crate::registry;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SYSTEM_CACHE_TTL_SECONDS: u64 = 30;

impl Engine {
    pub(crate) async fn probe_report(&self) -> Result<ProbeReport> {
        if let Some(report) = self.probe_cache.lock().ok().and_then(|cache| cache.clone()) {
            return Ok(report);
        }
        self.refresh_probe_report().await
    }

    pub(crate) async fn refresh_probe_report(&self) -> Result<ProbeReport> {
        let _permit = self.read_permit().await?;
        let specs = registry::requested(None)?;
        let mut set = tokio::task::JoinSet::new();
        for (index, spec) in specs.into_iter().enumerate() {
            set.spawn(async move {
                let status = probe::summarize_status(spec, &paths::probe_active(spec).await);
                (index, status)
            });
        }
        let mut indexed = Vec::new();
        while let Some(joined) = set.join_next().await {
            indexed.push(joined.map_err(|error| crate::Error::internal(error.to_string()))?);
        }
        indexed.sort_by_key(|(index, _)| *index);
        let tools = indexed.into_iter().map(|(_, status)| status).collect();
        let report = ProbeReport {
            tools,
            checked_at: clock::now(),
        };
        if let Ok(mut cache) = self.probe_cache.lock() {
            *cache = Some(report.clone());
        }
        Ok(report)
    }

    pub(crate) async fn system_report(&self) -> Result<SystemReport> {
        self.cached_system_report(None, false).await
    }

    pub(crate) async fn selected_report(
        &self,
        ids: &[String],
        force: bool,
    ) -> Result<SystemReport> {
        self.cached_system_report(Some(ids), force).await
    }

    pub(crate) async fn refresh_system_report(&self) -> Result<SystemReport> {
        *self.system_cache.lock().await = None;
        self.cached_system_report(None, true).await
    }

    pub(crate) async fn cached_system_report(
        &self,
        ids: Option<&[String]>,
        force: bool,
    ) -> Result<SystemReport> {
        let _permit = self.read_permit().await?;
        if force {
            self.backend_ctx.metadata.clear().await;
            self.update_cache
                .lock()
                .map_err(|_| crate::Error::internal("无法清除更新缓存。"))?
                .clear();
        }
        let specs = registry::requested(ids)?;
        let mut cache = self.system_cache.lock().await;
        if let Some((at, report)) = cache.as_ref()
            && !force
            && at.elapsed() < Duration::from_secs(SYSTEM_CACHE_TTL_SECONDS)
            && specs
                .iter()
                .all(|spec| report.tools.iter().any(|tool| tool.status.id == spec.id))
        {
            let mut selected = report.clone();
            selected
                .tools
                .retain(|tool| specs.iter().any(|spec| tool.status.id == spec.id));
            return Ok(selected);
        }
        let report = collect_report(&specs, Arc::new(BackendSet::discover().await)).await?;
        *cache = Some((Instant::now(), report.clone()));
        Ok(report)
    }
}

pub(crate) async fn collect_full_report(backends: Arc<BackendSet>) -> Result<SystemReport> {
    let specs = registry::requested(None)?;
    collect_report(&specs, backends).await
}

async fn collect_report(
    specs: &[&'static ToolSpec],
    backends: Arc<BackendSet>,
) -> Result<SystemReport> {
    if specs.is_empty() {
        return Ok(SystemReport {
            tools: Vec::new(),
            backends: backends.statuses(),
            checked_at: clock::now(),
            issue: None,
        });
    }

    let mut set = tokio::task::JoinSet::new();
    for (index, &spec) in specs.iter().enumerate() {
        let backends = backends.clone();
        set.spawn(async move {
            let result = match inspect_tool(spec, &backends).await {
                Ok(tool) => (tool, false),
                Err(error) => {
                    let executables = paths::probe_active(spec).await;
                    (
                        ToolReport {
                            status: probe::summarize_status(spec, &executables),
                            state: ToolState::Unavailable,
                            source: ToolSource::Unavailable,
                            owner: None,
                            available_backends: backends.available_for(spec),
                            executables,
                            packages: Vec::new(),
                            actions: Vec::new(),
                            can_select_version: false,
                            message: error.message,
                        },
                        true,
                    )
                }
            };
            (index, result)
        });
    }

    let mut indexed = Vec::with_capacity(specs.len());
    while let Some(joined) = set.join_next().await {
        indexed.push(joined.map_err(|error| crate::Error::internal(error.to_string()))?);
    }
    indexed.sort_by_key(|(index, _)| *index);

    let mut issue = None;
    let mut tools = Vec::with_capacity(indexed.len());
    for (_, (tool, had_error)) in indexed {
        if had_error {
            issue.get_or_insert_with(|| "部分工具的安装来源读取失败。".into());
        }
        tools.push(tool);
    }

    Ok(SystemReport {
        tools,
        backends: backends.statuses(),
        checked_at: clock::now(),
        issue,
    })
}

pub(crate) async fn inspect_tool(spec: &ToolSpec, backends: &BackendSet) -> Result<ToolReport> {
    if let Some(error) = backends.failure_for(spec) {
        return Err(error.clone());
    }
    let available_backends = backends.available_for(spec);
    let owners: Vec<(&Backend, Vec<Package>)> = backends
        .iter()
        .filter_map(|backend| {
            let packages = backend.installed_packages(spec);
            (!packages.is_empty()).then_some((backend, packages))
        })
        .collect();

    if owners.len() > 1 {
        return inspect_multi_backend(spec, owners, available_backends).await;
    }

    let (owner, packages) = match owners.into_iter().next() {
        Some((backend, packages)) => (Some(backend), packages),
        None => (None, Vec::new()),
    };

    let mut owned_files = Vec::new();
    let mut bundled_files = Vec::new();
    if let Some(backend) = owner {
        owned_files = backend.owned_files(spec, &packages).await?;
    } else {
        for &backend_id in &available_backends {
            if let Some(backend) = backends.get(backend_id) {
                bundled_files.extend(backend.owned_files(spec, &[]).await?);
            }
        }
    }

    let mut all_files: Vec<PathBuf> = Vec::with_capacity(owned_files.len() + bundled_files.len());
    all_files.extend(owned_files.iter().cloned());
    all_files.extend(bundled_files.iter().cloned());

    let mut executables = paths::scan(spec, &all_files).await?;
    let status = probe::summarize_status(spec, &executables);

    let owned_resolved: BTreeSet<_> = owned_files
        .iter()
        .map(|path| paths::resolved(path))
        .collect();
    let bundled_resolved: BTreeSet<_> = bundled_files
        .iter()
        .map(|path| paths::resolved(path))
        .collect();

    let version_manager = owner.is_some_and(|backend| {
        packages
            .iter()
            .any(|package| backend.is_version_manager_package(package))
    });

    for entry in &mut executables {
        let resolved = Path::new(&entry.resolved_path);
        entry.source = if let Some(backend) = owner
            && owned_resolved.contains(resolved)
        {
            if version_manager {
                ExecutableSource::VersionManager
            } else {
                ExecutableSource::Backend {
                    backend: backend.id(),
                }
            }
        } else if bundled_resolved.contains(resolved) {
            ExecutableSource::Bundled
        } else {
            ExecutableSource::External
        };
    }

    let has_external_exec = executables
        .iter()
        .any(|entry| entry.source == ExecutableSource::External);
    let present = !executables.is_empty() || !packages.is_empty();
    let usable = status.found;
    let bundled = present && packages.is_empty() && !has_external_exec;

    let source = if let Some(backend) = owner {
        if has_external_exec {
            ToolSource::Mixed {
                backend: backend.id(),
            }
        } else if version_manager {
            ToolSource::External
        } else {
            ToolSource::Managed {
                backend: backend.id(),
            }
        }
    } else if bundled {
        ToolSource::Bundled
    } else if has_external_exec {
        ToolSource::External
    } else if let Some(&default_backend) = available_backends.first() {
        ToolSource::Managed {
            backend: default_backend,
        }
    } else {
        ToolSource::Unavailable
    };

    let (base_state, base_message) = if present && !usable {
        (ToolState::Unavailable, "已安装，但命令不可用。")
    } else if has_external_exec && owner.is_some() {
        (
            ToolState::External,
            "同时存在其他来源的安装，禁止自动修改，请先通过原安装方式处理。",
        )
    } else if has_external_exec || version_manager {
        (ToolState::External, "由其他来源管理。")
    } else if bundled {
        (ToolState::External, "随其他软件安装。")
    } else if present {
        (ToolState::Installed, "")
    } else {
        (ToolState::Missing, "")
    };

    let mut actions = Vec::new();
    let mut auth_issue = None;
    if !present {
        let installable_backends: Vec<&Backend> = available_backends
            .iter()
            .filter_map(|id| backends.get(*id))
            .collect();
        if installable_backends
            .iter()
            .any(|backend| backend.authorization_issue().is_none())
        {
            actions.push(Action::Install);
        } else {
            auth_issue = installable_backends
                .iter()
                .find_map(|backend| backend.authorization_issue());
        }
    } else if let Some(backend) = owner {
        auth_issue = backend.authorization_issue();
        if auth_issue.is_none() && !has_external_exec {
            if !version_manager
                && usable
                && !packages.is_empty()
                && executables.iter().all(|entry| entry.issue.is_none())
            {
                actions.push(Action::Update);
            }
            if !packages.is_empty() && !version_manager {
                actions.push(Action::Uninstall);
            }
        }
    }

    let protected = owner.is_some_and(|backend| backend.is_protected_tool(spec, &packages));
    if protected {
        actions.clear();
    }

    let message = if protected {
        "系统组件，禁止修改。".into()
    } else if let Some(issue) = auth_issue {
        issue
    } else if !present && available_backends.is_empty() {
        "未发现受支持的包管理器。".into()
    } else {
        base_message.into()
    };

    let version_backend_ready = if let Some(backend) = owner {
        backend.capabilities().version_selection
    } else {
        available_backends
            .iter()
            .any(|id| id.capabilities().version_selection)
    };
    let can_select_version = version_backend_ready
        && (actions.contains(&Action::Install) || actions.contains(&Action::Update));

    Ok(ToolReport {
        status,
        state: if protected {
            ToolState::System
        } else {
            base_state
        },
        source,
        owner: owner.map(Backend::id),
        available_backends,
        executables,
        packages,
        actions,
        can_select_version,
        message,
    })
}

async fn inspect_multi_backend(
    spec: &ToolSpec,
    owners: Vec<(&Backend, Vec<Package>)>,
    available_backends: Vec<BackendId>,
) -> Result<ToolReport> {
    let mut owner_ids = Vec::with_capacity(owners.len());
    let mut all_packages = Vec::new();
    let mut owned_by_path: Vec<(BackendId, BTreeSet<PathBuf>)> = Vec::new();
    let mut all_files = Vec::new();

    for (backend, packages) in owners {
        owner_ids.push(backend.id());
        let files = backend.owned_files(spec, &packages).await?;
        let resolved: BTreeSet<_> = files.iter().map(|path| paths::resolved(path)).collect();
        owned_by_path.push((backend.id(), resolved));
        all_files.extend(files);
        all_packages.extend(packages);
    }

    let mut executables = paths::scan(spec, &all_files).await?;
    let status = probe::summarize_status(spec, &executables);
    for entry in &mut executables {
        let resolved = Path::new(&entry.resolved_path);
        entry.source = owned_by_path
            .iter()
            .find(|(_, set)| set.contains(resolved))
            .map_or(ExecutableSource::External, |(backend_id, _)| {
                ExecutableSource::Backend {
                    backend: *backend_id,
                }
            });
    }

    let names: Vec<_> = owner_ids.iter().map(|id| id.display_name()).collect();
    Ok(ToolReport {
        status,
        state: ToolState::External,
        source: ToolSource::MultiBackend {
            backends: owner_ids,
        },
        owner: None,
        available_backends,
        executables,
        packages: all_packages,
        actions: Vec::new(),
        can_select_version: false,
        message: format!(
            "{} 同时由多个来源安装（{}），请先通过原安装方式保留一个来源。",
            spec.name,
            names.join("、")
        ),
    })
}
