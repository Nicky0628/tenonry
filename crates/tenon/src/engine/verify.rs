use crate::backend::{Backend, BackendSet};
use crate::engine::inspect;
use crate::error::{Error, Result};
use crate::model::{
    Action, BatchPlan, ExecutionState, ItemOutcome, Outcome, Plan, SystemReport, ToolSource,
    VerificationState,
};
use crate::registry;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) async fn verify_batch(
    plan: &BatchPlan,
    overall_execution: ExecutionState,
    mut details: String,
    per_tool_execution: &BTreeMap<String, ExecutionState>,
) -> Result<Outcome> {
    // Discover after execution and use the same live package snapshot for both
    // executable ownership and target verification instead of discovering twice.
    let backends = Arc::new(BackendSet::discover().await);
    let report = match inspect::collect_full_report(backends.clone()).await {
        Ok(report) => Some(report),
        Err(error) => {
            details.push_str(&format!("\n无法确认工具状态：{error}"));
            None
        }
    };

    let mut items = Vec::with_capacity(plan.plans.len());
    for target in &plan.plans {
        let exec_state = per_tool_execution
            .get(&target.tool.status.id)
            .copied()
            .unwrap_or(overall_execution);
        let backend = backends.get(target.backend);
        let single = evaluate_single(target, exec_state, report.as_ref(), backend);
        items.push(ItemOutcome {
            tool_id: target.tool.status.id.clone(),
            name: target.tool.status.name.clone(),
            completed: single.completed,
            execution: single.execution,
            verification: single.verification,
            message: single.message,
        });
    }

    let all_passed = items
        .iter()
        .all(|item| item.verification == VerificationState::Passed);
    let overall_succeeded = matches!(
        overall_execution,
        ExecutionState::Succeeded | ExecutionState::NotNeeded
    );
    let completed = overall_succeeded && all_passed;

    let message = if items.len() == 1 {
        items[0].message.clone()
    } else if all_passed && overall_execution == ExecutionState::Unknown {
        "工具目前已达到预期状态，上次执行结果未能完整保存。".into()
    } else if completed {
        "操作完成，所选工具已确认。".into()
    } else {
        "部分结果需要确认，请查看各工具状态。".into()
    };

    let verification = if all_passed {
        VerificationState::Passed
    } else if items
        .iter()
        .any(|item| item.verification == VerificationState::Unknown)
    {
        VerificationState::Unknown
    } else {
        VerificationState::Failed
    };

    let error = if completed {
        None
    } else if overall_execution == ExecutionState::Failed {
        Some(Error::command_failed(message.clone()))
    } else {
        Some(Error::conflict(message.clone()))
    };

    Ok(Outcome {
        completed,
        message,
        details,
        report,
        execution: overall_execution,
        verification,
        items,
        error,
    })
}

struct SingleEval {
    completed: bool,
    execution: ExecutionState,
    verification: VerificationState,
    message: String,
}

fn evaluate_single(
    plan: &Plan,
    execution: ExecutionState,
    report: Option<&SystemReport>,
    backend: Option<&Backend>,
) -> SingleEval {
    let tool = report.and_then(|report| {
        report
            .tools
            .iter()
            .find(|tool| tool.status.id == plan.tool.status.id)
    });

    let inventory_complete = report.is_some()
        && tool.is_some_and(|tool| tool.source != ToolSource::Unavailable)
        && backend.is_some_and(|backend| backend.same_destination(plan));

    let targets_verified = backend.is_some_and(|backend| {
        let installed = backend.installed_packages_all();
        plan.preview.targets.iter().all(|expected| {
            let matches = |actual: &crate::model::Package| {
                backend.matches_installed_package(&expected.name, actual)
            };
            match &expected.candidate {
                Some(version) => installed
                    .iter()
                    .any(|actual| matches(actual) && actual.version == *version),
                None => !installed.iter().any(matches),
            }
        })
    });

    let verified = inventory_complete
        && tool.is_some_and(|tool| match plan.action {
            Action::Uninstall => {
                targets_verified
                    && plan.packages.iter().all(|name| {
                        !tool.packages.iter().any(|package| {
                            backend.is_some_and(|backend| {
                                backend.matches_installed_package(name, package)
                            })
                        })
                    })
            }
            Action::Install | Action::Update => {
                tool.status.found
                    && tool.owner == Some(plan.backend)
                    && tool.source
                        == ToolSource::Managed {
                            backend: plan.backend,
                        }
                    && targets_verified
                    && (plan.version.is_none()
                        || plan
                            .version
                            .as_deref()
                            .and_then(|value| value.split_once('='))
                            .and_then(|(_, version)| registry::extract_release(version))
                            .is_some_and(|expected| {
                                tool.status
                                    .version
                                    .as_deref()
                                    .and_then(registry::extract_release)
                                    .as_deref()
                                    == Some(expected.as_str())
                            }))
            }
        });

    let succeeded = matches!(
        execution,
        ExecutionState::Succeeded | ExecutionState::NotNeeded
    );
    let message = if execution == ExecutionState::Unknown {
        "命令执行结果未能确认，请结合目标校验与日志核对，勿直接重复执行。".into()
    } else if execution == ExecutionState::NotStarted {
        if verified {
            "本项尚未执行，但当前已处于目标状态。".into()
        } else {
            "前置步骤未完成，本项尚未执行。".into()
        }
    } else if !succeeded {
        if verified {
            "命令返回失败，但目标状态已达到预期。请查看日志，勿重复执行。".into()
        } else {
            "命令未成功完成。可重新校验当前状态，或重新检查计划后重试。".into()
        }
    } else if !verified {
        "命令成功，目标状态尚未确认。".into()
    } else {
        match plan.action {
            Action::Install => "安装成功，目标版本和可用性已确认。",
            Action::Update => "更新成功，目标版本和可用性已确认。",
            Action::Uninstall => "所列软件包已移除。",
        }
        .into()
    };

    let verification = if verified {
        VerificationState::Passed
    } else if inventory_complete {
        VerificationState::Failed
    } else {
        VerificationState::Unknown
    };

    SingleEval {
        completed: succeeded && verified,
        execution,
        verification,
        message,
    }
}
