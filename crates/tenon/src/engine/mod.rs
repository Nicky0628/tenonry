pub(crate) mod executor;
pub(crate) mod inspect;
pub(crate) mod plan;
pub(crate) mod tasks;
pub(crate) mod updates;
pub(crate) mod verify;
pub(crate) mod versions;

use crate::backend::BackendContext;
use crate::error::{Error, Result};
use crate::model::{BackendId, ProbeReport, SystemReport};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore, SemaphorePermit};

/// Internal shared service state behind the public `Tenon` facade.
#[derive(Clone)]
pub(crate) struct Engine {
    pub(crate) data_dir: PathBuf,
    pub(crate) read_limit: Arc<Semaphore>,
    pub(crate) write_limit: Arc<Semaphore>,
    pub(crate) backend_ctx: BackendContext,
    pub(crate) probe_cache: Arc<StdMutex<Option<ProbeReport>>>,
    pub(crate) system_cache: Arc<Mutex<Option<(Instant, SystemReport)>>>,
    pub(crate) update_cache: Arc<StdMutex<BTreeMap<BackendId, updates::BackendUpdateCache>>>,
    pub(crate) tasks: tasks::TaskStore,
}

impl Engine {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        let backend_ctx = BackendContext::new(data_dir.clone());
        Self {
            data_dir,
            read_limit: Arc::new(Semaphore::new(4)),
            write_limit: Arc::new(Semaphore::new(1)),
            backend_ctx,
            probe_cache: Arc::default(),
            system_cache: Arc::default(),
            update_cache: Arc::default(),
            tasks: tasks::TaskStore::default(),
        }
    }

    pub(crate) fn lock_path(&self) -> PathBuf {
        self.data_dir.join("tenon.lock")
    }

    pub(crate) fn data_directory(&self) -> &Path {
        &self.data_dir
    }

    pub(crate) async fn read_permit(&self) -> Result<SemaphorePermit<'_>> {
        self.read_limit
            .acquire()
            .await
            .map_err(|_| Error::internal("环境读取服务已关闭。"))
    }

    pub(crate) fn write_permit(&self) -> Result<OwnedSemaphorePermit> {
        self.write_limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::busy("已有写入操作正在进行，请完成后再试。"))
    }

    pub(crate) async fn invalidate_reports(&self) {
        if let Ok(mut cache) = self.probe_cache.lock() {
            *cache = None;
        }
        *self.system_cache.lock().await = None;
    }
}
