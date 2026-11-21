use crate::backend::{Backend, BackendContext};
use crate::error::{Error, Result};
use crate::infra::{
    clock,
    lock::{FileLock, SharedLock},
    record,
};
use crate::model::{BackendId, CatalogStatus};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard};

const FRESH_SECONDS: i64 = 30 * 60;
const FAILURE_COOLDOWN_SECONDS: u64 = 60;

#[derive(Default)]
pub(crate) struct CatalogState {
    failure: Option<(Instant, Error)>,
}

/// Per-backend in-process catalog refresh locks and failure backoff tracking.
#[derive(Clone, Default)]
pub(crate) struct CatalogRegistry {
    states: Arc<StdMutex<BTreeMap<BackendId, Arc<Mutex<CatalogState>>>>>,
}

impl CatalogRegistry {
    pub(crate) fn lock_for(&self, id: BackendId) -> Arc<Mutex<CatalogState>> {
        let mut map = self
            .states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        map.entry(id).or_default().clone()
    }
}

/// Holds exclusive or shared catalog locks while a backend query or plan uses its index.
pub(crate) struct CatalogLease {
    pub(crate) backend: Backend,
    pub(crate) status: CatalogStatus,
    _guard: OwnedMutexGuard<CatalogState>,
    _file_lock: FileLock,
}

pub(crate) struct CatalogSnapshot {
    pub(crate) backend: Backend,
    pub(crate) revision: String,
    pub(crate) cached: bool,
    _lease: Option<CatalogLease>,
}

fn is_fresh(status: &CatalogStatus, id: BackendId) -> bool {
    status.backend == id
        && DateTime::parse_from_rfc3339(&status.checked_at).is_ok_and(|at| {
            (0..FRESH_SECONDS).contains(&(Utc::now() - at.with_timezone(&Utc)).num_seconds())
        })
}

fn backend_catalog_dir(ctx: &BackendContext, id: BackendId) -> PathBuf {
    ctx.data_dir.join("catalog").join(id.as_str())
}

/// Acquires and refreshes (when stale or forced) a backend's package catalog.
pub(crate) async fn acquire(
    ctx: &BackendContext,
    id: BackendId,
    force: bool,
) -> Result<CatalogLease> {
    if !id.capabilities().catalog_refresh {
        return Err(Error::unsupported(format!(
            "{} 按包实时查询元数据，不需要刷新整机软件源。",
            id.display_name()
        )));
    }
    let state_mutex = ctx.catalogs.lock_for(id);
    let mut guard = state_mutex.lock_owned().await;
    if !force
        && let Some((at, error)) = &guard.failure
        && at.elapsed() < Duration::from_secs(FAILURE_COOLDOWN_SECONDS)
    {
        return Err(error.clone());
    }

    let mut backend = Backend::detect_one(id)
        .await?
        .ok_or_else(|| Error::not_found(format!("当前系统未检测到 {}。", id.display_name())))?;

    let directory = backend_catalog_dir(ctx, id);
    std::fs::create_dir_all(&directory)
        .map_err(|error| Error::persistence(format!("无法创建索引目录：{error}")))?;
    let file_lock = FileLock::acquire_exclusive(
        &directory.join("refresh.lock"),
        "另一个进程正在获取更新信息，请稍后重试。",
    )?;

    let stamp = directory.join("catalog.json");
    let previous: Option<CatalogStatus> = record::load(&stamp, 16 * 1024)?;
    let fresh = previous.as_ref().is_some_and(|status| is_fresh(status, id));

    backend.configure_catalog(&directory)?;

    let status = if fresh && !force {
        previous.ok_or_else(|| Error::persistence("更新记录缺失。"))?
    } else {
        if stamp.exists() {
            std::fs::remove_file(&stamp).map_err(|error| Error::persistence(error.to_string()))?;
        }
        match backend.refresh_catalog(&directory).await {
            Ok(message) => {
                let status = CatalogStatus {
                    backend: id,
                    checked_at: clock::now(),
                    message,
                };
                record::save(&stamp, &status)?;
                guard.failure = None;
                status
            }
            Err(error) => {
                guard.failure = Some((Instant::now(), error.clone()));
                return Err(error);
            }
        }
    };

    backend.set_catalog_basis(Some(format!("软件源索引：{}", status.checked_at)));

    Ok(CatalogLease {
        backend,
        status,
        _guard: guard,
        _file_lock: file_lock,
    })
}

/// Reads cached catalog metadata without starting or waiting for a network refresh.
pub(crate) async fn snapshot(
    ctx: &BackendContext,
    mut backend: Backend,
) -> Result<CatalogSnapshot> {
    let id = backend.id();
    let capabilities = backend.capabilities();
    if !capabilities.catalog_refresh {
        return Ok(CatalogSnapshot {
            revision: backend.local_catalog_revision()?,
            backend,
            cached: false,
            _lease: None,
        });
    }

    let directory = backend_catalog_dir(ctx, id);
    let state_mutex = ctx.catalogs.lock_for(id);
    let mut refreshing = false;

    if let Ok(guard) = state_mutex.try_lock_owned() {
        match FileLock::try_acquire_shared(&directory.join("refresh.lock"))? {
            SharedLock::Acquired(file_lock) => {
                let status: Option<CatalogStatus> =
                    record::load(&directory.join("catalog.json"), 16 * 1024)?;
                if let Some(status) = status.filter(|status| status.backend == id) {
                    let cached = !is_fresh(&status, id);
                    backend.configure_catalog(&directory)?;
                    backend.set_catalog_basis(Some(format!("软件源索引：{}", status.checked_at)));
                    let revision = status.checked_at.clone();
                    let lease = CatalogLease {
                        backend: backend.clone(),
                        status,
                        _guard: guard,
                        _file_lock: file_lock,
                    };
                    return Ok(CatalogSnapshot {
                        backend,
                        revision,
                        cached,
                        _lease: Some(lease),
                    });
                }
            }
            SharedLock::Contended => refreshing = true,
            SharedLock::Missing => {}
        }
    } else {
        refreshing = true;
    }

    if refreshing && !capabilities.isolated_catalog {
        return Err(Error::busy("更新信息正在同步，请稍后再试。"));
    }

    let revision = backend.local_catalog_revision()?;
    backend.set_catalog_basis(Some("本机软件源索引".into()));
    Ok(CatalogSnapshot {
        backend,
        revision,
        cached: true,
        _lease: None,
    })
}
