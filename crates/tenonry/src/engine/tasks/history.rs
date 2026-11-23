use super::classify_recovered;
use crate::backend::BackendSet;
use crate::engine::{Engine, verify};
use crate::error::{Error, Result};
use crate::infra::lock::FileLock;
use crate::model::{ExecutionState, Operation, OperationPhase, OperationState};
use std::collections::BTreeMap;

impl Engine {
    pub(crate) fn operations(&self) -> Result<Vec<Operation>> {
        self.read_operations(false)
    }

    pub(super) fn read_operations(&self, owns_lock: bool) -> Result<Vec<Operation>> {
        let mut state = self
            .tasks
            .0
            .lock()
            .map_err(|_| Error::internal("无法读取环境任务状态。"))?;
        if state.active.is_none()
            && !state
                .journal
                .entries
                .iter()
                .any(|entry| entry.operation.persistence_issue.is_some())
        {
            state.load(self.data_directory())?;
        }
        let running_elsewhere = !owns_lock
            && state.active.is_none()
            && state
                .journal
                .entries
                .iter()
                .any(|entry| entry.operation.state == OperationState::Running)
            && FileLock::is_held(&self.lock_path())?;

        Ok(state
            .journal
            .entries
            .iter()
            .rev()
            .map(|entry| {
                if let Some((id, started)) = &state.active
                    && id == &entry.operation.id
                {
                    let mut operation = entry.operation.clone();
                    operation.elapsed_ms = started.elapsed().as_millis() as u64;
                    operation
                } else if running_elsewhere {
                    entry.operation.clone()
                } else {
                    classify_recovered(&entry.operation)
                }
            })
            .collect())
    }

    pub(crate) fn operation(&self, id: &str) -> Result<Operation> {
        self.operations()?
            .into_iter()
            .find(|operation| operation.id == id)
            .ok_or_else(|| Error::not_found("找不到本次环境任务，请重新读取操作记录。"))
    }

    pub(crate) fn latest_operation(&self) -> Result<Option<Operation>> {
        Ok(self.operations()?.into_iter().next())
    }

    pub(crate) async fn recheck_operation(&self, id: &str) -> Result<Operation> {
        let _permit = self.write_permit()?;
        let _lock = FileLock::acquire_exclusive(
            &self.lock_path(),
            "已有系统环境操作正在进行，请稍后再试。",
        )?;
        let operation = self
            .read_operations(true)?
            .into_iter()
            .find(|operation| operation.id == id)
            .ok_or_else(|| Error::not_found("找不到本次环境任务。"))?;
        if operation.state == OperationState::Running {
            return Err(Error::busy("操作还在进行，请稍后再确认。"));
        }
        if operation.phase == OperationPhase::Interrupted {
            BackendSet::ensure_all_idle().await?;
        }

        let plan = {
            let state = self
                .tasks
                .0
                .lock()
                .map_err(|_| Error::internal("无法读取任务。"))?;
            state
                .journal
                .entries
                .iter()
                .find(|entry| entry.operation.id == id)
                .ok_or_else(|| Error::not_found("找不到任务。"))?
                .plan
                .clone()
        };

        let outcome = operation
            .outcome
            .ok_or_else(|| Error::internal("操作结果缺失。"))?;
        if matches!(
            outcome.execution,
            ExecutionState::NotStarted | ExecutionState::NotNeeded
        ) {
            return self.operation(id);
        }

        let per_tool_execution: BTreeMap<String, ExecutionState> = outcome
            .items
            .iter()
            .map(|item| (item.tool_id.clone(), item.execution))
            .collect();
        let result = verify::verify_batch(
            &plan,
            outcome.execution,
            outcome.details,
            &per_tool_execution,
        )
        .await?;

        {
            let mut state = self
                .tasks
                .0
                .lock()
                .map_err(|_| Error::internal("无法保存任务。"))?;
            let entry = state
                .journal
                .entries
                .iter_mut()
                .find(|entry| entry.operation.id == id)
                .ok_or_else(|| Error::not_found("找不到任务。"))?;
            entry.operation.state = if result.completed {
                OperationState::Completed
            } else {
                OperationState::NeedsAttention
            };
            entry.operation.phase = OperationPhase::Finished;
            entry.operation.message = result.message.clone();
            if !result.items.is_empty() {
                entry.operation.tools = result.items.clone();
            }
            entry.operation.outcome = Some(result);
            if let Err(error) = state.save(self.data_directory()) {
                if let Some(entry) = state
                    .journal
                    .entries
                    .iter_mut()
                    .find(|entry| entry.operation.id == id)
                {
                    entry.operation.persistence_issue = Some(error.message.clone());
                }
                return Err(error);
            }
        }
        self.invalidate_reports().await;
        self.operation(id)
    }
}
