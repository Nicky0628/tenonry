//! Tenon: cross-platform development tool discovery, multi-backend package
//! coordination, and verifiable task execution.
//!
//! Callers supply a data directory where task journals (`operations.json`),
//! per-backend catalog caches (`catalog/`), and the cross-process lock
//! (`tenon.lock`) are persisted. Constructing [`Tenon`] performs no disk I/O.
//! Clones share in-memory caches and active task state.

mod backend;
mod engine;
mod error;
mod infra;
mod model;
mod registry;

pub use error::{Error, ErrorKind, Result};
pub use model::{
    Action, BackendCapabilities, BackendId, BackendScope, BackendStatus, BatchPlan, BatchPlanInput,
    BatchStartInput, CatalogStatus, Change, Command, Executable, ExecutableSource, ExecutionState,
    ItemOutcome, Operation, OperationPhase, OperationState, Outcome, Package, Plan, PlanInput,
    Preview, PreviewState, ProbeReport, StartInput, SystemReport, ToolChoice, ToolReport,
    ToolSource, ToolState, ToolStatus, TransactionPlan, UpdateCheck, UpdateState,
    VerificationState, VersionInfo, VersionList,
};
pub use registry::{available_tools, is_known_tool};

use backend::catalog;
use engine::Engine;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Primary service entry point for Tenon.
#[derive(Clone)]
pub struct Tenon {
    engine: Engine,
}

impl Tenon {
    /// Creates a service bound to `directory` without creating or modifying files.
    /// Relative paths are resolved once against the current working directory.
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self> {
        let directory = directory.into();
        let directory = if directory.is_absolute() {
            directory
        } else {
            std::env::current_dir()
                .map_err(|error| Error::persistence(format!("无法确定环境数据目录：{error}")))?
                .join(directory)
        };
        Ok(Self {
            engine: Engine::new(directory),
        })
    }

    /// Shares an application's concurrency budget across reads and mutations.
    /// Configure before cloning or executing operations.
    pub fn with_operation_limits(mut self, reads: Arc<Semaphore>, writes: Arc<Semaphore>) -> Self {
        self.engine.read_limit = reads;
        self.engine.write_limit = writes;
        self
    }

    pub fn data_directory(&self) -> &Path {
        self.engine.data_directory()
    }

    /// Lightweight `PATH` probe for all supported tools on this OS (cached).
    pub async fn probe_report(&self) -> Result<ProbeReport> {
        self.engine.probe_report().await
    }

    /// Forces a fresh lightweight `PATH` probe.
    pub async fn refresh_probe_report(&self) -> Result<ProbeReport> {
        self.engine.refresh_probe_report().await
    }

    /// Full environment inventory including backend discovery and package ownership.
    pub async fn system_report(&self) -> Result<SystemReport> {
        self.engine.system_report().await
    }

    /// Inventory restricted to the given tool IDs.
    pub async fn selected_report(&self, ids: &[String], force: bool) -> Result<SystemReport> {
        self.engine.selected_report(ids, force).await
    }

    /// Clears cached reports and re-inspects the full environment.
    pub async fn refresh_system_report(&self) -> Result<SystemReport> {
        self.engine.refresh_system_report().await
    }

    /// Refreshes the package catalog for a backend that supports catalog refresh.
    pub async fn refresh_catalog(&self, backend: BackendId, force: bool) -> Result<CatalogStatus> {
        Ok(catalog::acquire(&self.engine.backend_ctx, backend, force)
            .await?
            .status)
    }

    /// Queries available versions for a tool (optionally targeting a specific backend).
    pub async fn list_versions(
        &self,
        tool_id: String,
        backend: Option<BackendId>,
    ) -> Result<VersionList> {
        self.engine.list_versions(tool_id, backend).await
    }

    /// Checks available updates for the specified tools across their owning backends.
    pub async fn check_updates(&self, tool_ids: Vec<String>) -> Result<Vec<UpdateCheck>> {
        self.engine.check_updates(tool_ids).await
    }

    /// Checks available update for a single tool.
    pub async fn check_update(&self, tool_id: String) -> Result<UpdateCheck> {
        self.engine.check_update(tool_id).await
    }

    /// Generates a reviewed, version-locked single-tool plan.
    pub async fn prepare_plan(&self, input: PlanInput) -> Result<Plan> {
        self.engine.prepare_plan(input).await
    }

    /// Generates a reviewed batch update plan grouped by backend transactions.
    pub async fn prepare_batch(&self, input: BatchPlanInput) -> Result<BatchPlan> {
        self.engine.prepare_batch(input).await
    }

    /// Starts a reviewed plan and awaits its completion.
    pub async fn apply_plan(&self, confirmation: Plan) -> Result<Outcome> {
        self.engine.apply_plan(confirmation).await
    }

    /// Starts a reviewed plan asynchronously with a generated request ID.
    pub async fn start_plan(&self, confirmation: Plan) -> Result<Operation> {
        self.engine.start_plan(confirmation).await
    }

    /// Starts a reviewed plan asynchronously with a caller-supplied idempotent request ID.
    pub async fn start_request(&self, input: StartInput) -> Result<Operation> {
        self.engine.start_request(input).await
    }

    /// Starts a reviewed batch plan asynchronously.
    pub async fn start_batch(&self, input: BatchStartInput) -> Result<Operation> {
        self.engine.start_batch(input).await
    }

    /// Lists persisted operation history, newest first.
    pub fn operations(&self) -> Result<Vec<Operation>> {
        self.engine.operations()
    }

    /// Fetches one operation record by ID.
    pub fn operation(&self, id: &str) -> Result<Operation> {
        self.engine.operation(id)
    }

    /// Fetches the most recent operation record, if any.
    pub fn latest_operation(&self) -> Result<Option<Operation>> {
        self.engine.latest_operation()
    }

    /// Re-verifies a completed or interrupted operation against live system state.
    pub async fn recheck_operation(&self, id: &str) -> Result<Operation> {
        self.engine.recheck_operation(id).await
    }
}
