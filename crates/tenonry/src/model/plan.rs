use super::{BackendId, ToolReport};
use serde::{Deserialize, Serialize};

/// Supported package lifecycle actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Action {
    /// Install a tool that is not currently managed.
    Install,
    /// Upgrade or change the version of an installed tool.
    Update,
    /// Remove a managed tool.
    Uninstall,
}

/// A concrete command step to execute on the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Command {
    /// Executable path or program name.
    pub program: String,
    /// Command-line arguments passed without shell expansion.
    pub arguments: Vec<String>,
    /// Whether this step requires administrative privileges.
    pub administrator: bool,
}

/// Package-level change predicted by a backend preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    /// Package name as known to the backend.
    pub name: String,
    /// Currently installed version, if any.
    pub installed: Option<String>,
    /// Target version after the operation, if any.
    pub candidate: Option<String>,
    /// Whether this package is an automatic dependency rather than the primary target.
    pub dependency: bool,
}

/// Whether a previewed operation would actually apply changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum PreviewState {
    /// Concrete package changes are available to apply.
    Available,
    /// The system already satisfies the requested state; no changes are needed.
    Current,
}

/// Structured preflight preview for an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preview {
    /// Whether changes are available or already up to date.
    pub state: PreviewState,
    /// Short description of the backend source or simulation used for preflight.
    pub basis: String,
    /// Primary target package transitions.
    pub targets: Vec<Change>,
    /// Full set of predicted package additions, upgrades, or removals.
    pub changes: Vec<Change>,
    /// Raw or summarized preflight details for user review.
    pub details: String,
}

/// Request to generate a single-tool operation plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanInput {
    /// Registry identifier of the tool (for example, `"node"` or `"python"`).
    pub tool_id: String,
    /// Lifecycle action to plan.
    pub action: Action,
    /// Optional explicit backend selection (used when multiple backends can install a tool).
    #[serde(default)]
    pub backend: Option<BackendId>,
    /// Optional target version (for backends supporting version selection).
    #[serde(default)]
    pub version: Option<String>,
}

/// Reviewed and version-locked single-tool operation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    /// Snapshot of the tool state captured during planning.
    pub tool: ToolReport,
    /// Planned lifecycle action.
    pub action: Action,
    /// Backend selected to execute this plan.
    pub backend: BackendId,
    /// Backend package names involved in this plan.
    pub packages: Vec<String>,
    /// Version-locked command sequence to execute.
    pub commands: Vec<Command>,
    /// Human-readable impact summary for confirmation dialogs.
    pub impact: String,
    /// Structured preflight preview.
    pub preview: Preview,
    /// Explicit target version requested by the caller, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A backend-scoped execution unit within a batch or single operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionPlan {
    /// Backend executing this transaction.
    pub backend: BackendId,
    /// Lifecycle action shared by all items in this transaction.
    pub action: Action,
    /// Registry tool IDs covered by this transaction.
    pub tool_ids: Vec<String>,
    /// Backend package names covered by this transaction.
    pub packages: Vec<String>,
    /// Concrete command sequence for this transaction.
    pub commands: Vec<Command>,
    /// Merged preflight preview for this transaction.
    pub preview: Preview,
}

impl TransactionPlan {
    /// Wraps a single-tool [`Plan`] as a one-item [`TransactionPlan`].
    pub fn from_single(plan: &Plan) -> Self {
        Self {
            backend: plan.backend,
            action: plan.action,
            tool_ids: vec![plan.tool.status.id.clone()],
            packages: plan.packages.clone(),
            commands: plan.commands.clone(),
            preview: plan.preview.clone(),
        }
    }
}

/// Request to generate a batch update plan across multiple tools.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchPlanInput {
    /// Registry tool IDs to include in the batch update plan.
    pub tool_ids: Vec<String>,
}

/// Reviewed batch plan grouping tool plans into per-backend transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchPlan {
    /// Individual tool plans included in this batch.
    pub plans: Vec<Plan>,
    /// Per-backend transactions executed to fulfill `plans`.
    pub transactions: Vec<TransactionPlan>,
}
