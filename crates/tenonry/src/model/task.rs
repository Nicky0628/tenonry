use super::{Action, BatchPlan, Plan, SystemReport};
use crate::error::Error;
use serde::{Deserialize, Serialize};

/// Coarse lifecycle state of a recorded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum OperationState {
    /// The operation is actively executing or verifying.
    Running,
    /// Execution and post-run verification completed cleanly.
    Completed,
    /// Execution failed, verification failed, or the task was interrupted and requires review.
    NeedsAttention,
}

/// Fine-grained execution phase of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum OperationPhase {
    /// Re-checking plan validity and environment state before launching commands.
    Checking,
    /// Waiting for administrative elevation or authorization.
    Authorizing,
    /// Downloading packages from the backend repository.
    Downloading,
    /// Installing or upgrading packages.
    Installing,
    /// Running post-install configuration steps.
    Configuring,
    /// Removing packages.
    Removing,
    /// Running backend command steps without a more specific phase signal.
    Executing,
    /// Inspecting live system state to verify the expected outcome.
    Verifying,
    /// Task reached a terminal state.
    Finished,
    /// Task was interrupted before finishing (for example, by a process crash).
    Interrupted,
}

/// Execution outcome of a command sequence or transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ExecutionState {
    /// Rejected during pre-execution checks before any command started.
    NotStarted,
    /// Target state was already satisfied; no commands needed to run.
    NotNeeded,
    /// All scheduled commands exited successfully.
    Succeeded,
    /// One or more commands failed during execution.
    Failed,
    /// Execution result could not be confirmed (for example, after an interruption).
    Unknown,
}

/// Post-execution verification state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum VerificationState {
    /// Live system inspection matched the expected post-operation state.
    Passed,
    /// Live system inspection did not match the expected post-operation state.
    Failed,
    /// Verification could not be completed conclusively.
    Unknown,
}

/// Per-tool outcome within a single or batch operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemOutcome {
    /// Registry identifier of the tool.
    pub tool_id: String,
    /// Human-readable display name of the tool.
    pub name: String,
    /// Whether this item succeeded and passed verification.
    pub completed: bool,
    /// Command execution result for this item.
    pub execution: ExecutionState,
    /// Live verification result for this item.
    pub verification: VerificationState,
    /// Human-readable status summary for this item.
    pub message: String,
}

/// Final outcome of an executed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Whether the entire operation completed and passed verification.
    pub completed: bool,
    /// Human-readable summary message.
    pub message: String,
    /// Extended execution and verification detail text.
    pub details: String,
    /// Refreshed [`SystemReport`] collected during verification, when available.
    pub report: Option<SystemReport>,
    /// Overall command execution outcome.
    pub execution: ExecutionState,
    /// Overall post-execution verification outcome.
    pub verification: VerificationState,
    /// Per-tool outcomes for single or batch operations.
    #[serde(default)]
    pub items: Vec<ItemOutcome>,
    /// Structured error recorded when the operation failed or was rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

/// Persisted operation record and live task progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// Unique operation identifier (UUID string).
    pub id: String,
    /// Primary tool identifier, or `"batch"` for multi-tool batch operations.
    pub tool_id: String,
    /// Human-readable operation target name.
    pub name: String,
    /// Lifecycle action performed by this operation.
    pub action: Action,
    /// Requested target version, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Per-tool outcome breakdown.
    pub tools: Vec<ItemOutcome>,
    /// Coarse lifecycle state of the operation.
    pub state: OperationState,
    /// Current or final execution phase.
    pub phase: OperationPhase,
    /// Human-readable progress or completion summary.
    pub message: String,
    /// RFC 3339 UTC timestamp when the operation started.
    pub started_at: String,
    /// Elapsed execution time in milliseconds.
    pub elapsed_ms: u64,
    /// Captured command output log.
    pub log: String,
    /// Final outcome once the operation finishes or is re-checked.
    pub outcome: Option<Outcome>,
    /// Non-fatal persistence error encountered while saving the journal, if any.
    pub persistence_issue: Option<String>,
}

/// Idempotent start request for a single reviewed plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInput {
    /// Caller-supplied UUID used to deduplicate retried submissions.
    pub request_id: String,
    /// Complete reviewed [`Plan`] returned by [`crate::Tenonry::prepare_plan`].
    pub confirmation: Plan,
}

/// Idempotent start request for a reviewed batch plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchStartInput {
    /// Caller-supplied UUID used to deduplicate retried submissions.
    pub request_id: String,
    /// Complete reviewed [`BatchPlan`] returned by [`crate::Tenonry::prepare_batch`].
    pub confirmation: BatchPlan,
}
