use crate::backend::{Backend, BackendSet, CandidateTargets, catalog};
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::infra::clock;
use crate::model::{
    Action, BackendId, Package, Preview, PreviewState, ToolReport, UpdateCheck, UpdateState,
};
use crate::registry;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(crate) struct BackendUpdateCache {
    catalog_revision: String,
    installed: Vec<Package>,
    candidates: CandidateTargets,
    checked_at: String,
}

impl Engine {
    pub(crate) async fn check_updates(&self, tool_ids: Vec<String>) -> Result<Vec<UpdateCheck>> {
        let mut unique = BTreeSet::new();
        if tool_ids.len() > registry::max_tool_count()
            || tool_ids
                .iter()
                .any(|id| !unique.insert(id) || registry::find_available(id).is_err())
        {
            return Err(Error::invalid_input("请选择有效的工具，不能重复选择。"));
        }
        if tool_ids.is_empty() {
            return Ok(Vec::new());
        }

        let report = self.cached_system_report(Some(&tool_ids), false).await?;
        let _permit = self.read_permit().await?;
        let backends = BackendSet::discover().await;

        let mut by_backend: BTreeMap<BackendId, Vec<(String, ToolReport)>> = BTreeMap::new();
        let mut results: BTreeMap<String, UpdateCheck> = BTreeMap::new();
        let timestamp = clock::now();

        for id in &tool_ids {
            let Some(tool) = report.tools.iter().find(|tool| tool.status.id == *id) else {
                results.insert(
                    id.clone(),
                    failed_check(
                        id.clone(),
                        None,
                        "工具状态缺失，请刷新。".into(),
                        &timestamp,
                    ),
                );
                continue;
            };
            if !tool.actions.contains(&Action::Update) {
                results.insert(
                    id.clone(),
                    failed_check(id.clone(), tool.owner, tool.action_error(), &timestamp),
                );
                continue;
            }
            let Some(backend_id) = tool.owner else {
                results.insert(
                    id.clone(),
                    failed_check(
                        id.clone(),
                        None,
                        "安装来源缺失，请刷新。".into(),
                        &timestamp,
                    ),
                );
                continue;
            };
            by_backend
                .entry(backend_id)
                .or_default()
                .push((id.clone(), tool.clone()));
        }

        // Query each backend group in parallel without hardcoding any backend kind.
        let mut join_set = tokio::task::JoinSet::new();
        for (backend_id, group) in by_backend {
            let engine = self.clone();
            let backend = backends.require(backend_id).cloned();
            join_set.spawn(async move {
                let outcome = match backend {
                    Ok(backend) => engine.check_backend_group(backend, &group).await,
                    Err(error) => Err(error),
                };
                (backend_id, group, outcome)
            });
        }

        while let Some(joined) = join_set.join_next().await {
            let (backend_id, group, outcome) =
                joined.map_err(|error| Error::internal(error.to_string()))?;
            match outcome {
                Ok(checks) => {
                    for check in checks {
                        results.insert(check.tool_id.clone(), check);
                    }
                }
                Err(error) => {
                    let now = clock::now();
                    for (tool_id, _) in group {
                        results.insert(
                            tool_id.clone(),
                            failed_check(tool_id, Some(backend_id), error.message.clone(), &now),
                        );
                    }
                }
            }
        }

        tool_ids
            .into_iter()
            .map(|id| {
                results
                    .remove(&id)
                    .ok_or_else(|| Error::internal("缺少工具更新结果。"))
            })
            .collect()
    }

    pub(crate) async fn check_update(&self, tool_id: String) -> Result<UpdateCheck> {
        match self.check_updates(vec![tool_id.clone()]).await {
            Ok(mut checks) => checks
                .pop()
                .ok_or_else(|| Error::internal("工具更新结果缺失。")),
            Err(error) => Ok(failed_check(tool_id, None, error.message, &clock::now())),
        }
    }

    async fn check_backend_group(
        &self,
        backend: Backend,
        group: &[(String, ToolReport)],
    ) -> Result<Vec<UpdateCheck>> {
        let backend_id = backend.id();
        let snapshot = catalog::snapshot(&self.backend_ctx, backend).await?;
        let backend = &snapshot.backend;

        let tool_packages: Vec<Result<Vec<String>>> = group
            .iter()
            .map(|(tool_id, tool)| {
                let spec = registry::find_available(tool_id)?;
                if tool.packages != backend.installed_packages(spec) {
                    return Err(Error::plan_expired(
                        "安装状态已变化，请刷新工具状态后重试。",
                    ));
                }
                let names = backend.update_packages(spec, &tool.packages);
                if names.is_empty() {
                    return Err(Error::not_found("没有可以查询更新的软件包。"));
                }
                Ok(names)
            })
            .collect();

        let all_names: Vec<String> = tool_packages
            .iter()
            .filter_map(|item| item.as_ref().ok())
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let cached = self
            .update_cache
            .lock()
            .map_err(|_| Error::internal("无法读取更新缓存。"))?
            .get(&backend_id)
            .filter(|cache| {
                cache.catalog_revision == snapshot.revision
                    && cache.installed == backend.installed_packages_all()
                    && all_names
                        .iter()
                        .all(|name| cache.candidates.contains_key(name))
            })
            .cloned();

        let (candidates, checked_at) = if let Some(cache) = cached {
            (cache.candidates, cache.checked_at)
        } else {
            let candidates = backend.check_updates(&self.backend_ctx, &all_names).await?;
            let checked_at = clock::now();
            if !all_names.is_empty() && candidates.values().all(Result::is_ok) {
                self.update_cache
                    .lock()
                    .map_err(|_| Error::internal("无法保存更新缓存。"))?
                    .insert(
                        backend_id,
                        BackendUpdateCache {
                            catalog_revision: snapshot.revision.clone(),
                            installed: backend.installed_packages_all().to_vec(),
                            candidates: candidates.clone(),
                            checked_at: checked_at.clone(),
                        },
                    );
            }
            (candidates, checked_at)
        };

        let basis = backend.catalog_basis().unwrap_or("软件源索引").to_owned();

        Ok(group
            .iter()
            .zip(tool_packages)
            .map(|((tool_id, _), packages)| {
                let targets = packages.and_then(|names| {
                    names
                        .iter()
                        .map(|name| {
                            candidates.get(name).cloned().ok_or_else(|| {
                                Error::internal(format!("缺少 {name} 的更新信息。"))
                            })?
                        })
                        .collect::<Result<Vec<_>>>()
                });
                match targets {
                    Ok(targets) => {
                        let has_update = targets
                            .iter()
                            .any(|target| target.installed != target.candidate);
                        let (state, preview_state) = if has_update {
                            (UpdateState::Available, PreviewState::Available)
                        } else {
                            (UpdateState::Current, PreviewState::Current)
                        };
                        UpdateCheck {
                            tool_id: tool_id.clone(),
                            state,
                            backend: Some(backend_id),
                            message: String::new(),
                            checked_at: checked_at.clone(),
                            cached: snapshot.cached,
                            preview: Some(Preview {
                                state: preview_state,
                                targets,
                                changes: Vec::new(),
                                details: String::new(),
                                basis: basis.clone(),
                            }),
                        }
                    }
                    Err(error) => failed_check(
                        tool_id.clone(),
                        Some(backend_id),
                        error.message,
                        &checked_at,
                    ),
                }
            })
            .collect())
    }
}

fn failed_check(
    tool_id: String,
    backend: Option<BackendId>,
    message: String,
    checked_at: &str,
) -> UpdateCheck {
    UpdateCheck {
        tool_id,
        state: UpdateState::Failed,
        backend,
        preview: None,
        message,
        checked_at: checked_at.to_owned(),
        cached: false,
    }
}
