mod backend;
mod plan;
mod spec;
mod task;
mod tool;
mod update;
mod version;

pub use backend::{BackendCapabilities, BackendId, BackendScope, BackendStatus};
pub use plan::{
    Action, BatchPlan, BatchPlanInput, Change, Command, Plan, PlanInput, Preview, PreviewState,
    TransactionPlan,
};
pub(crate) use spec::{Identity, Platforms, ToolSpec};
pub use task::{
    BatchStartInput, ExecutionState, ItemOutcome, Operation, OperationPhase, OperationState,
    Outcome, StartInput, VerificationState,
};
pub use tool::{
    Executable, ExecutableSource, Package, ProbeReport, SystemReport, ToolChoice, ToolReport,
    ToolSource, ToolState, ToolStatus,
};
pub use update::{CatalogStatus, UpdateCheck, UpdateState};
pub use version::{VersionInfo, VersionList};
