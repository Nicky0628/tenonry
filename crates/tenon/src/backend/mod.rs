pub(crate) mod apt;
pub(crate) mod catalog;
pub(crate) mod common;
pub(crate) mod dnf;
pub(crate) mod npm;
pub(crate) mod pacman;
pub(crate) mod winget;

use crate::error::{Error, Result};
use crate::model::{
    Action, BackendCapabilities, BackendId, BackendScope, BackendStatus, Change, OperationPhase,
    Package, Plan, ToolReport, ToolSpec, TransactionPlan, VersionList,
};
use catalog::CatalogRegistry;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(crate) type CandidateTargets = BTreeMap<String, Result<Change>>;

const METADATA_TTL_SECONDS: u64 = 30 * 60;
const METADATA_MAX_ENTRIES: usize = 128;

/// Generic key-value TTL cache shared by backends that query per-package metadata.
#[derive(Clone, Default)]
pub(crate) struct MetadataCache {
    entries: Arc<Mutex<BTreeMap<String, (Instant, Value)>>>,
}

impl MetadataCache {
    pub(crate) async fn get(&self, key: &str) -> Option<Value> {
        let guard = self.entries.lock().await;
        guard
            .get(key)
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(METADATA_TTL_SECONDS))
            .map(|(_, value)| value.clone())
    }

    pub(crate) async fn insert(&self, key: String, value: Value) {
        let mut guard = self.entries.lock().await;
        if guard.len() >= METADATA_MAX_ENTRIES {
            guard.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(METADATA_TTL_SECONDS));
        }
        if guard.len() >= METADATA_MAX_ENTRIES {
            guard.clear();
        }
        guard.insert(key, (Instant::now(), value));
    }

    pub(crate) async fn clear(&self) {
        self.entries.lock().await.clear();
    }
}

/// Shared runtime resources passed to backend operations.
#[derive(Clone)]
pub(crate) struct BackendContext {
    pub(crate) data_dir: PathBuf,
    pub(crate) catalogs: CatalogRegistry,
    pub(crate) metadata: MetadataCache,
}

impl BackendContext {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            catalogs: CatalogRegistry::default(),
            metadata: MetadataCache::default(),
        }
    }
}

/// Uniform contract implemented by every package manager backend.
pub(crate) trait BackendDriver {
    fn id(&self) -> BackendId;

    fn capabilities(&self) -> BackendCapabilities {
        self.id().capabilities()
    }

    fn installed_packages_all(&self) -> &[Package];

    fn supports_tool(&self, spec: &ToolSpec) -> bool;

    fn installed_packages(&self, spec: &ToolSpec) -> Vec<Package>;

    async fn owned_files(&self, spec: &ToolSpec, packages: &[Package]) -> Result<Vec<PathBuf>>;

    fn is_protected_tool(&self, _spec: &ToolSpec, _packages: &[Package]) -> bool {
        false
    }

    fn is_version_manager_package(&self, package: &Package) -> bool {
        matches!(package.name.as_str(), "rustup" | "Rustlang.Rustup")
    }

    fn authorization_issue(&self) -> Option<String> {
        None
    }

    fn update_packages(&self, _spec: &ToolSpec, packages: &[Package]) -> Vec<String> {
        packages
            .iter()
            .map(|package| package.name.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn configure_catalog(&mut self, _directory: &Path) -> Result<()> {
        Ok(())
    }

    fn set_catalog_basis(&mut self, _basis: Option<String>) {}

    fn catalog_basis(&self) -> Option<&str> {
        None
    }

    async fn refresh_catalog(&self, _directory: &Path) -> Result<String> {
        Err(Error::unsupported(format!(
            "{} 不需要刷新软件源索引。",
            self.id().display_name()
        )))
    }

    fn local_catalog_revision(&self) -> Result<String> {
        Ok(format!("local-{}", chrono::Utc::now().timestamp() / 60))
    }

    async fn check_updates(
        &self,
        ctx: &BackendContext,
        packages: &[String],
    ) -> Result<CandidateTargets>;

    async fn list_versions(
        &self,
        _ctx: &BackendContext,
        spec: &ToolSpec,
        tool: &ToolReport,
    ) -> Result<VersionList> {
        Ok(VersionList {
            tool_id: spec.id.into(),
            backend: Some(self.id()),
            versions: Vec::new(),
            message: if tool.message.is_empty() {
                "此工具不支持指定版本。".into()
            } else {
                tool.message.clone()
            },
        })
    }

    async fn prepare_plan(
        &self,
        ctx: &BackendContext,
        spec: &ToolSpec,
        tool: ToolReport,
        action: Action,
        version: Option<String>,
    ) -> Result<Plan>;

    async fn prepare_batch_transaction(
        &self,
        ctx: &BackendContext,
        plans: &[Plan],
    ) -> Result<TransactionPlan>;

    async fn validate_destination(&self, _transaction: &TransactionPlan) -> Result<()> {
        Ok(())
    }

    fn same_destination(&self, _plan: &Plan) -> bool {
        true
    }

    fn matches_installed_package(&self, expected: &str, actual: &Package) -> bool {
        actual.name == expected
    }

    /// Checks whether uninstalling `spec` is blocked by packages managed by this backend.
    fn check_dependent_removal(&self, _spec: &ToolSpec) -> Result<()> {
        Ok(())
    }
}

/// Static dispatch wrapper over all registered backends.
#[derive(Debug, Clone)]
pub(crate) enum Backend {
    Apt(apt::AptBackend),
    Dnf(dnf::DnfBackend),
    Pacman(pacman::PacmanBackend),
    Winget(winget::WingetBackend),
    Npm(npm::NpmBackend),
}

macro_rules! dispatch {
    ($self:expr, $inner:ident => $body:expr) => {
        match $self {
            Backend::Apt($inner) => $body,
            Backend::Dnf($inner) => $body,
            Backend::Pacman($inner) => $body,
            Backend::Winget($inner) => $body,
            Backend::Npm($inner) => $body,
        }
    };
}

macro_rules! dispatch_boxed {
    ($self:expr, $ret:ty, $inner:ident => $fut:expr) => {{
        let future: Pin<Box<dyn Future<Output = $ret> + Send + '_>> = match $self {
            Backend::Apt($inner) => Box::pin($fut),
            Backend::Dnf($inner) => Box::pin($fut),
            Backend::Pacman($inner) => Box::pin($fut),
            Backend::Winget($inner) => Box::pin($fut),
            Backend::Npm($inner) => Box::pin($fut),
        };
        future.await
    }};
}

impl Backend {
    pub(crate) fn platform_candidates() -> &'static [BackendId] {
        if cfg!(windows) {
            &[BackendId::Winget, BackendId::Npm]
        } else if cfg!(target_os = "linux") {
            &[
                BackendId::Apt,
                BackendId::Dnf,
                BackendId::Pacman,
                BackendId::Npm,
            ]
        } else {
            &[]
        }
    }

    pub(crate) async fn detect_one(id: BackendId) -> Result<Option<Self>> {
        let future: Pin<Box<dyn Future<Output = Result<Option<Self>>> + Send>> = match id {
            BackendId::Apt => {
                Box::pin(async { Ok(apt::AptBackend::detect().await?.map(Backend::Apt)) })
            }
            BackendId::Dnf => {
                Box::pin(async { Ok(dnf::DnfBackend::detect().await?.map(Backend::Dnf)) })
            }
            BackendId::Pacman => {
                Box::pin(async { Ok(pacman::PacmanBackend::detect().await?.map(Backend::Pacman)) })
            }
            BackendId::Winget => {
                Box::pin(async { Ok(winget::WingetBackend::detect().await?.map(Backend::Winget)) })
            }
            BackendId::Npm => {
                Box::pin(async { Ok(npm::NpmBackend::detect().await?.map(Backend::Npm)) })
            }
        };
        future.await
    }

    pub(crate) fn supports_tool_for(id: BackendId, spec: &ToolSpec) -> bool {
        match id {
            BackendId::Apt => apt::AptBackend::supports_tool_id(spec.id),
            BackendId::Dnf => dnf::DnfBackend::supports_tool_id(spec.id),
            BackendId::Pacman => pacman::PacmanBackend::supports_tool_id(spec.id),
            BackendId::Winget => winget::WingetBackend::supports_tool_id(spec.id),
            BackendId::Npm => npm::NpmBackend::supports_tool_id(spec.id),
        }
    }

    pub(crate) fn depends_on_tool_for(id: BackendId, spec: &ToolSpec) -> bool {
        match id {
            BackendId::Npm => spec.id == "node",
            _ => false,
        }
    }

    pub(crate) fn parse_progress_for(
        id: BackendId,
        line: &str,
    ) -> Option<(OperationPhase, &'static str)> {
        match id {
            BackendId::Apt => apt::AptBackend::parse_progress_line(line),
            _ => None,
        }
    }

    pub(crate) fn is_benign_exit_for(id: BackendId, action: Action, status: ExitStatus) -> bool {
        match id {
            BackendId::Winget => winget::WingetBackend::is_benign_exit_code(action, status),
            _ => false,
        }
    }

    pub(crate) async fn check_busy_for(id: BackendId) -> Result<bool> {
        match id {
            BackendId::Apt => apt::AptBackend::check_busy().await,
            BackendId::Dnf => dnf::DnfBackend::check_busy().await,
            BackendId::Pacman => pacman::PacmanBackend::check_busy().await,
            BackendId::Winget => winget::WingetBackend::check_busy().await,
            BackendId::Npm => npm::NpmBackend::check_busy().await,
        }
    }

    pub(crate) fn id(&self) -> BackendId {
        dispatch!(self, b => b.id())
    }

    pub(crate) fn capabilities(&self) -> BackendCapabilities {
        dispatch!(self, b => b.capabilities())
    }

    pub(crate) fn installed_packages_all(&self) -> &[Package] {
        dispatch!(self, b => b.installed_packages_all())
    }

    pub(crate) fn supports_tool(&self, spec: &ToolSpec) -> bool {
        dispatch!(self, b => b.supports_tool(spec))
    }

    pub(crate) fn installed_packages(&self, spec: &ToolSpec) -> Vec<Package> {
        dispatch!(self, b => b.installed_packages(spec))
    }

    pub(crate) async fn owned_files(
        &self,
        spec: &ToolSpec,
        packages: &[Package],
    ) -> Result<Vec<PathBuf>> {
        dispatch_boxed!(self, Result<Vec<PathBuf>>, b => b.owned_files(spec, packages))
    }

    pub(crate) fn is_protected_tool(&self, spec: &ToolSpec, packages: &[Package]) -> bool {
        dispatch!(self, b => b.is_protected_tool(spec, packages))
    }

    pub(crate) fn is_version_manager_package(&self, package: &Package) -> bool {
        dispatch!(self, b => b.is_version_manager_package(package))
    }

    pub(crate) fn authorization_issue(&self) -> Option<String> {
        dispatch!(self, b => b.authorization_issue())
    }

    pub(crate) fn update_packages(&self, spec: &ToolSpec, packages: &[Package]) -> Vec<String> {
        dispatch!(self, b => b.update_packages(spec, packages))
    }

    pub(crate) fn configure_catalog(&mut self, directory: &Path) -> Result<()> {
        dispatch!(self, b => b.configure_catalog(directory))
    }

    pub(crate) fn set_catalog_basis(&mut self, basis: Option<String>) {
        dispatch!(self, b => b.set_catalog_basis(basis))
    }

    pub(crate) fn catalog_basis(&self) -> Option<&str> {
        dispatch!(self, b => b.catalog_basis())
    }

    pub(crate) async fn refresh_catalog(&self, directory: &Path) -> Result<String> {
        dispatch_boxed!(self, Result<String>, b => b.refresh_catalog(directory))
    }

    pub(crate) fn local_catalog_revision(&self) -> Result<String> {
        dispatch!(self, b => b.local_catalog_revision())
    }

    pub(crate) async fn check_updates(
        &self,
        ctx: &BackendContext,
        packages: &[String],
    ) -> Result<CandidateTargets> {
        dispatch_boxed!(self, Result<CandidateTargets>, b => b.check_updates(ctx, packages))
    }

    pub(crate) async fn list_versions(
        &self,
        ctx: &BackendContext,
        spec: &ToolSpec,
        tool: &ToolReport,
    ) -> Result<VersionList> {
        dispatch_boxed!(self, Result<VersionList>, b => b.list_versions(ctx, spec, tool))
    }

    pub(crate) async fn prepare_plan(
        &self,
        ctx: &BackendContext,
        spec: &ToolSpec,
        tool: ToolReport,
        action: Action,
        version: Option<String>,
    ) -> Result<Plan> {
        dispatch_boxed!(self, Result<Plan>, b => b.prepare_plan(ctx, spec, tool, action, version))
    }

    pub(crate) async fn prepare_batch_transaction(
        &self,
        ctx: &BackendContext,
        plans: &[Plan],
    ) -> Result<TransactionPlan> {
        dispatch_boxed!(self, Result<TransactionPlan>, b => b.prepare_batch_transaction(ctx, plans))
    }

    pub(crate) async fn validate_destination(&self, transaction: &TransactionPlan) -> Result<()> {
        dispatch_boxed!(self, Result<()>, b => b.validate_destination(transaction))
    }

    pub(crate) fn same_destination(&self, plan: &Plan) -> bool {
        dispatch!(self, b => b.same_destination(plan))
    }

    pub(crate) fn matches_installed_package(&self, expected: &str, actual: &Package) -> bool {
        dispatch!(self, b => b.matches_installed_package(expected, actual))
    }

    pub(crate) fn check_dependent_removal(&self, spec: &ToolSpec) -> Result<()> {
        dispatch!(self, b => b.check_dependent_removal(spec))
    }
}

/// Discovered backends on the current machine, supporting coexistence and fault isolation.
pub(crate) struct BackendSet {
    backends: BTreeMap<BackendId, Backend>,
    failures: BTreeMap<BackendId, Error>,
    statuses: Vec<BackendStatus>,
}

impl BackendSet {
    pub(crate) async fn discover() -> Self {
        let mut set = tokio::task::JoinSet::new();
        for &id in Backend::platform_candidates() {
            set.spawn(async move { (id, Backend::detect_one(id).await) });
        }

        let mut results = BTreeMap::new();
        while let Some(joined) = set.join_next().await {
            if let Ok((id, outcome)) = joined {
                results.insert(id, outcome);
            }
        }

        let mut backends = BTreeMap::new();
        let mut failures = BTreeMap::new();
        let mut statuses = Vec::new();
        for &id in Backend::platform_candidates() {
            let outcome = results.remove(&id).unwrap_or(Ok(None));
            match outcome {
                Ok(Some(backend)) => {
                    statuses.push(BackendStatus {
                        id,
                        name: id.display_name().into(),
                        capabilities: id.capabilities(),
                        available: true,
                        issue: None,
                    });
                    backends.insert(id, backend);
                }
                Ok(None) => {}
                Err(error) => {
                    statuses.push(BackendStatus {
                        id,
                        name: id.display_name().into(),
                        capabilities: id.capabilities(),
                        available: false,
                        issue: Some(error.message.clone()),
                    });
                    failures.insert(id, error);
                }
            }
        }

        Self {
            backends,
            failures,
            statuses,
        }
    }

    pub(crate) fn statuses(&self) -> Vec<BackendStatus> {
        self.statuses.clone()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Backend> {
        self.backends.values()
    }

    pub(crate) fn get(&self, id: BackendId) -> Option<&Backend> {
        self.backends.get(&id)
    }

    pub(crate) fn replace(&mut self, backend: Backend) {
        self.backends.insert(backend.id(), backend);
    }

    pub(crate) fn require(&self, id: BackendId) -> Result<&Backend> {
        if let Some(backend) = self.backends.get(&id) {
            return Ok(backend);
        }
        if let Some(error) = self.failures.get(&id) {
            return Err(error.clone());
        }
        Err(Error::not_found(format!(
            "当前系统未检测到 {}。",
            id.display_name()
        )))
    }

    pub(crate) fn available_for(&self, spec: &ToolSpec) -> Vec<BackendId> {
        let mut list: Vec<BackendId> = self
            .backends
            .values()
            .filter(|backend| backend.supports_tool(spec))
            .map(Backend::id)
            .collect();
        // Prefer ecosystem backends first when a tool is explicitly supported there (e.g. pnpm via npm).
        list.sort_by_key(|id| match id.scope() {
            BackendScope::Ecosystem => 0,
            BackendScope::System => 1,
        });
        list
    }

    pub(crate) fn failure_for(&self, spec: &ToolSpec) -> Option<&Error> {
        self.failures
            .iter()
            .find(|(id, _)| Backend::supports_tool_for(**id, spec))
            .map(|(_, error)| error)
    }

    /// Ensures uninstalling `spec` will not orphan packages in any dependent backend.
    /// A failed probe of a dependent backend also blocks removal so we never skip
    /// protection when its inventory cannot be verified.
    pub(crate) fn validate_removal(&self, spec: &ToolSpec) -> Result<()> {
        for (id, error) in &self.failures {
            if Backend::depends_on_tool_for(*id, spec) {
                return Err(Error::conflict(format!(
                    "无法确认 {} 全局安装状态（{}），已暂停卸载 {}。",
                    id.display_name(),
                    error.message,
                    spec.name
                )));
            }
        }
        for backend in self.backends.values() {
            backend.check_dependent_removal(spec)?;
        }
        Ok(())
    }

    /// Resolves which backend should handle an action for `spec`.
    pub(crate) fn resolve_for_tool(
        &self,
        spec: &ToolSpec,
        requested: Option<BackendId>,
    ) -> Result<&Backend> {
        if let Some(error) = self.failure_for(spec) {
            return Err(error.clone());
        }
        let owners: Vec<_> = self
            .backends
            .values()
            .filter(|backend| !backend.installed_packages(spec).is_empty())
            .collect();

        if owners.len() > 1 {
            let names: Vec<_> = owners
                .iter()
                .map(|backend| backend.id().display_name())
                .collect();
            return Err(Error::conflict(format!(
                "{} 同时由多个来源安装（{}），请先通过原安装方式保留一个来源。",
                spec.name,
                names.join("、")
            )));
        }

        if let Some(owner) = owners.into_iter().next() {
            if let Some(requested_id) = requested
                && requested_id != owner.id()
            {
                return Err(Error::conflict(format!(
                    "{} 当前由 {} 管理，不能切换为 {} 操作。",
                    spec.name,
                    owner.id().display_name(),
                    requested_id.display_name()
                )));
            }
            return Ok(owner);
        }

        if let Some(requested_id) = requested {
            let backend = self.require(requested_id)?;
            if !backend.supports_tool(spec) {
                return Err(Error::unsupported(format!(
                    "{} 暂不支持安装 {}。",
                    requested_id.display_name(),
                    spec.name
                )));
            }
            return Ok(backend);
        }

        if let Some(preferred_id) = self.available_for(spec).into_iter().next()
            && let Some(backend) = self.backends.get(&preferred_id)
        {
            return Ok(backend);
        }

        if let Some(error) = self.failures.values().next() {
            return Err(error.clone());
        }
        Err(Error::not_found(format!(
            "当前系统没有找到可管理 {} 的受支持包管理器。",
            spec.name
        )))
    }

    pub(crate) async fn ensure_all_idle() -> Result<()> {
        let mut set = tokio::task::JoinSet::new();
        for &id in Backend::platform_candidates() {
            set.spawn(async move {
                Ok::<(BackendId, bool), Error>((id, Backend::check_busy_for(id).await?))
            });
        }
        while let Some(joined) = set.join_next().await {
            let (id, busy) = joined.map_err(|error| Error::internal(error.to_string()))??;
            if busy {
                return Err(Error::busy(format!(
                    "{} 仍在运行，请等待它结束后再确认状态。",
                    id.display_name()
                )));
            }
        }
        Ok(())
    }
}
