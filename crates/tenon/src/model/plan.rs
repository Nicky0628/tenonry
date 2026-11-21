use super::{BackendId, ToolReport};
use serde::{Deserialize, Serialize};

/// Supported package lifecycle actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Install,
    Update,
    Uninstall,
}

/// A concrete command step to execute on the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Command {
    pub program: String,
    pub arguments: Vec<String>,
    pub administrator: bool,
}

/// Package-level change predicted by a backend preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub name: String,
    pub installed: Option<String>,
    pub candidate: Option<String>,
    pub dependency: bool,
}

/// Whether a previewed operation would actually apply changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewState {
    Available,
    Current,
}

/// Structured preflight preview for an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preview {
    pub state: PreviewState,
    pub basis: String,
    pub targets: Vec<Change>,
    pub changes: Vec<Change>,
    pub details: String,
}

/// Request to generate a single-tool operation plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanInput {
    pub tool_id: String,
    pub action: Action,
    #[serde(default)]
    pub backend: Option<BackendId>,
    #[serde(default)]
    pub version: Option<String>,
}

/// Reviewed and version-locked single-tool operation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub tool: ToolReport,
    pub action: Action,
    pub backend: BackendId,
    pub packages: Vec<String>,
    pub commands: Vec<Command>,
    pub impact: String,
    pub preview: Preview,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A backend-scoped execution unit within a batch or single operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionPlan {
    pub backend: BackendId,
    pub action: Action,
    pub tool_ids: Vec<String>,
    pub packages: Vec<String>,
    pub commands: Vec<Command>,
    pub preview: Preview,
}

impl TransactionPlan {
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
    pub tool_ids: Vec<String>,
}

/// Reviewed batch plan grouping tool plans into per-backend transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchPlan {
    pub plans: Vec<Plan>,
    pub transactions: Vec<TransactionPlan>,
}
