use super::{Action, BatchPlan, Plan, SystemReport};
use crate::error::Error;
use serde::{Deserialize, Serialize};

/// Coarse lifecycle state of a recorded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationState {
    Running,
    Completed,
    NeedsAttention,
}

/// Fine-grained execution phase of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationPhase {
    Checking,
    Authorizing,
    Downloading,
    Installing,
    Configuring,
    Removing,
    Executing,
    Verifying,
    Finished,
    Interrupted,
}

/// Execution outcome of a command sequence or transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionState {
    NotStarted,
    NotNeeded,
    Succeeded,
    Failed,
    Unknown,
}

/// Post-execution verification state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationState {
    Passed,
    Failed,
    Unknown,
}

/// Per-tool outcome within a single or batch operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemOutcome {
    pub tool_id: String,
    pub name: String,
    pub completed: bool,
    pub execution: ExecutionState,
    pub verification: VerificationState,
    pub message: String,
}

/// Final outcome of an executed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub completed: bool,
    pub message: String,
    pub details: String,
    pub report: Option<SystemReport>,
    pub execution: ExecutionState,
    pub verification: VerificationState,
    #[serde(default)]
    pub items: Vec<ItemOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

/// Persisted operation record and live task progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub tool_id: String,
    pub name: String,
    pub action: Action,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub tools: Vec<ItemOutcome>,
    pub state: OperationState,
    pub phase: OperationPhase,
    pub message: String,
    pub started_at: String,
    pub elapsed_ms: u64,
    pub log: String,
    pub outcome: Option<Outcome>,
    pub persistence_issue: Option<String>,
}

/// Idempotent start request for a single reviewed plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInput {
    pub request_id: String,
    pub confirmation: Plan,
}

/// Idempotent start request for a reviewed batch plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchStartInput {
    pub request_id: String,
    pub confirmation: BatchPlan,
}
