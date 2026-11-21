use crate::backend::{BackendSet, catalog};
use crate::engine::{Engine, inspect};
use crate::error::{Error, Result};
use crate::model::{
    Action, BackendId, BatchPlan, BatchPlanInput, Plan, PlanInput, TransactionPlan,
};
use crate::registry;
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub(crate) async fn prepare_plan(&self, input: PlanInput) -> Result<Plan> {
        let _permit = self.read_permit().await?;
        let (plan, _leases) = prepare_single(self, input).await?;
        Ok(plan)
    }

    pub(crate) async fn prepare_batch(&self, input: BatchPlanInput) -> Result<BatchPlan> {
        let _permit = self.read_permit().await?;
        let inputs: Vec<_> = input
            .tool_ids
            .into_iter()
            .map(|tool_id| PlanInput {
                tool_id,
                action: Action::Update,
                backend: None,
                version: None,
            })
            .collect();
        let (batch, _leases) = prepare_batch_internal(self, inputs).await?;
        Ok(batch)
    }
}

async fn prepare_single(
    engine: &Engine,
    input: PlanInput,
) -> Result<(Plan, Vec<catalog::CatalogLease>)> {
    let spec = registry::find_available(&input.tool_id)?;
    let mut backends = BackendSet::discover().await;
    if input.action == Action::Uninstall {
        backends.validate_removal(spec)?;
    }

    let target_backend_id = backends.resolve_for_tool(spec, input.backend)?.id();
    let mut leases = Vec::new();
    if target_backend_id.capabilities().catalog_refresh
        && input.action != Action::Uninstall
        && input.version.is_none()
    {
        let lease = catalog::acquire(&engine.backend_ctx, target_backend_id, false).await?;
        backends.replace(lease.backend.clone());
        leases.push(lease);
    }

    let tool = inspect::inspect_tool(spec, &backends).await?;
    let backend = backends.resolve_for_tool(spec, input.backend)?;
    if !tool.actions.contains(&input.action) {
        return Err(tool.action_rejection(backend.authorization_issue()));
    }
    if input.action == Action::Uninstall && input.version.is_some() {
        return Err(Error::invalid_input("卸载时不需要选择版本。"));
    }

    let plan = backend
        .prepare_plan(&engine.backend_ctx, spec, tool, input.action, input.version)
        .await?;
    Ok((plan, leases))
}

pub(crate) async fn prepare_batch_internal(
    engine: &Engine,
    inputs: Vec<PlanInput>,
) -> Result<(BatchPlan, Vec<catalog::CatalogLease>)> {
    let mut unique = BTreeSet::new();
    if inputs.is_empty()
        || inputs.len() > registry::max_tool_count()
        || inputs.iter().any(|input| !unique.insert(&input.tool_id))
    {
        return Err(Error::invalid_input("请选择需要操作的工具，不能重复选择。"));
    }

    let mut backends = BackendSet::discover().await;
    let mut catalog_targets = BTreeSet::new();
    for input in &inputs {
        let spec = registry::find_available(&input.tool_id)?;
        if input.action == Action::Uninstall {
            backends.validate_removal(spec)?;
        }
        let backend_id = backends.resolve_for_tool(spec, input.backend)?.id();
        if backend_id.capabilities().catalog_refresh
            && input.action != Action::Uninstall
            && input.version.is_none()
        {
            catalog_targets.insert(backend_id);
        }
    }

    let mut leases = Vec::new();
    for backend_id in catalog_targets {
        let lease = catalog::acquire(&engine.backend_ctx, backend_id, false).await?;
        backends.replace(lease.backend.clone());
        leases.push(lease);
    }

    let mut plans = Vec::with_capacity(inputs.len());
    for input in inputs {
        let spec = registry::find_available(&input.tool_id)?;
        let tool = inspect::inspect_tool(spec, &backends).await?;
        let backend = backends.resolve_for_tool(spec, input.backend)?;
        if !tool.actions.contains(&input.action) {
            return Err(tool.action_rejection(backend.authorization_issue()));
        }
        if input.action == Action::Uninstall && input.version.is_some() {
            return Err(Error::invalid_input("卸载时不需要选择版本。"));
        }
        let plan = backend
            .prepare_plan(&engine.backend_ctx, spec, tool, input.action, input.version)
            .await?;
        plans.push(plan);
    }

    let mut grouped: BTreeMap<BackendId, Vec<Plan>> = BTreeMap::new();
    for plan in &plans {
        grouped.entry(plan.backend).or_default().push(plan.clone());
    }

    let mut transactions = Vec::new();
    for (backend_id, group) in grouped {
        let can_merge = group.len() > 1
            && backend_id.capabilities().batch_transaction
            && group
                .iter()
                .all(|plan| plan.action == Action::Update && plan.version.is_none());
        if !can_merge {
            transactions.extend(group.iter().map(TransactionPlan::from_single));
            continue;
        }
        let backend = backends.require(backend_id)?;
        transactions.push(
            backend
                .prepare_batch_transaction(&engine.backend_ctx, &group)
                .await?,
        );
    }

    Ok((
        BatchPlan {
            plans,
            transactions,
        },
        leases,
    ))
}
