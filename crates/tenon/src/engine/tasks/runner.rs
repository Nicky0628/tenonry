use super::Progress;
use super::journal::{JournalEntry, MAX_ENTRIES, MAX_STARTED_IDS, OPERATIONS_FILE};
use crate::backend::BackendSet;
use crate::engine::{Engine, executor, plan, verify};
use crate::error::{Error, Result};
use crate::infra::{clock, lock::FileLock};
use crate::model::{
    BatchPlan, BatchStartInput, ExecutionState, ItemOutcome, Operation, OperationPhase,
    OperationState, Outcome, Plan, PlanInput, StartInput, TransactionPlan, VerificationState,
};
use crate::registry;
use std::time::{Duration, Instant};

impl Engine {
    pub(crate) async fn apply_plan(&self, confirmation: Plan) -> Result<Outcome> {
        let operation = self.start_plan(confirmation).await?;
        loop {
            let current = self.operation(&operation.id)?;
            if let Some(outcome) = current.outcome {
                if outcome.execution == ExecutionState::NotStarted
                    && let Some(error) = outcome.error
                {
                    return Err(error);
                }
                return Ok(outcome);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub(crate) async fn start_plan(&self, plan: Plan) -> Result<Operation> {
        self.start_request(StartInput {
            request_id: uuid::Uuid::new_v4().to_string(),
            confirmation: plan,
        })
        .await
    }

    pub(crate) async fn start_request(&self, input: StartInput) -> Result<Operation> {
        let transaction = TransactionPlan::from_single(&input.confirmation);
        self.start_batch(BatchStartInput {
            request_id: input.request_id,
            confirmation: BatchPlan {
                plans: vec![input.confirmation],
                transactions: vec![transaction],
            },
        })
        .await
    }

    pub(crate) async fn start_batch(&self, input: BatchStartInput) -> Result<Operation> {
        let id = uuid::Uuid::parse_str(&input.request_id)
            .map_err(|_| Error::invalid_input("环境任务标识无效。"))?
            .to_string();
        if let Some(current) = self
            .operations()?
            .into_iter()
            .find(|operation| operation.id == id)
        {
            return Ok(current);
        }

        let permit = self.write_permit()?;
        let lock = FileLock::acquire_exclusive(
            &self.lock_path(),
            "已有系统环境操作正在进行，请稍后再试。",
        )?;
        let history = self.read_operations(true)?;
        if let Some(current) = history.iter().find(|operation| operation.id == id) {
            return Ok(current.clone());
        }
        if history
            .iter()
            .any(|operation| operation.phase == OperationPhase::Interrupted)
        {
            return Err(Error::conflict(
                "上次操作尚未确认，请先在操作记录中确认当前状态。",
            ));
        }

        let batch = input.confirmation;
        if batch.plans.is_empty()
            || batch.plans.len() > registry::max_tool_count()
            || batch.transactions.is_empty()
        {
            return Err(Error::invalid_input("操作范围无效。"));
        }

        let tools: Vec<_> = batch
            .plans
            .iter()
            .map(|plan| ItemOutcome {
                tool_id: plan.tool.status.id.clone(),
                name: plan.tool.status.name.clone(),
                completed: false,
                execution: ExecutionState::NotStarted,
                verification: VerificationState::Unknown,
                message: "等待处理".into(),
            })
            .collect();

        let action = batch.plans[0].action;
        let version = if batch.plans.len() == 1 {
            batch.plans[0].version.clone()
        } else {
            None
        };
        let operation = Operation {
            id: id.clone(),
            tool_id: tools[0].tool_id.clone(),
            name: tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>()
                .join("、"),
            action,
            version,
            tools,
            state: OperationState::Running,
            phase: OperationPhase::Checking,
            message: "正在确认操作…".into(),
            started_at: clock::now(),
            elapsed_ms: 0,
            log: String::new(),
            outcome: None,
            persistence_issue: None,
        };

        {
            let mut state = self
                .tasks
                .0
                .lock()
                .map_err(|_| Error::internal("无法保存环境任务状态。"))?;
            let mut next = state.journal.clone();
            if next.started_ids.len() >= MAX_STARTED_IDS {
                return Err(Error::persistence(
                    "环境任务记录已达到上限，请备份后整理记录。",
                ));
            }
            if !next.started_ids.insert(id.clone()) {
                return Err(Error::conflict("此任务已执行过，请重新查看当前工具状态。"));
            }
            next.entries.push(JournalEntry {
                operation: operation.clone(),
                plan: batch.clone(),
            });
            if next.entries.len() > MAX_ENTRIES {
                next.entries.remove(0);
            }
            next.save(self.data_directory())?;
            state.journal = next;
            state.loaded = true;
            state.disk_stamp = std::fs::metadata(self.data_directory().join(OPERATIONS_FILE))
                .and_then(|metadata| metadata.modified())
                .ok();
            state.active = Some((id.clone(), Instant::now()));
        }

        let progress = Progress {
            tasks: self.tasks.clone(),
            id,
            directory: self.data_directory().to_path_buf(),
        };
        let engine = self.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _lock = lock;
            let worker = progress.clone();
            let result = tokio::spawn(async move { run_batch(&engine, &batch, &worker).await })
                .await
                .unwrap_or_else(|error| {
                    Err(Error::internal(format!(
                        "操作意外中断，请确认当前状态：{error}"
                    )))
                });
            progress.finish(result);
        });

        Ok(operation)
    }
}

async fn run_batch(engine: &Engine, confirmed: &BatchPlan, progress: &Progress) -> Result<Outcome> {
    progress.phase(OperationPhase::Checking, "正在确认所选工具…");
    let inputs: Vec<PlanInput> = confirmed
        .plans
        .iter()
        .map(|plan| PlanInput {
            tool_id: plan.tool.status.id.clone(),
            action: plan.action,
            backend: Some(plan.backend),
            version: plan.version.clone(),
        })
        .collect();
    let (current, _leases) = plan::prepare_batch_internal(engine, inputs).await?;
    if &current != confirmed {
        return Err(Error::plan_expired(
            "工具或更新信息已变化，请重新确认后再继续。",
        ));
    }

    if current
        .transactions
        .iter()
        .all(|transaction| transaction.commands.is_empty())
    {
        return Ok(Outcome {
            completed: true,
            message: "暂无更新，可以继续使用。".into(),
            details: String::new(),
            report: None,
            execution: ExecutionState::NotNeeded,
            verification: VerificationState::Passed,
            items: current
                .plans
                .iter()
                .map(|plan| ItemOutcome {
                    tool_id: plan.tool.status.id.clone(),
                    name: plan.tool.status.name.clone(),
                    completed: true,
                    execution: ExecutionState::NotNeeded,
                    verification: VerificationState::Passed,
                    message: "暂无更新".into(),
                })
                .collect(),
            error: None,
        });
    }

    // Persist the execution boundary before any child process modifies installed software.
    progress.checkpoint()?;

    let backends = BackendSet::discover().await;
    for transaction in &current.transactions {
        let backend = backends.require(transaction.backend)?;
        backend.validate_destination(transaction).await?;
    }

    let execution = executor::execute_transactions(&current.transactions, progress).await;
    engine.invalidate_reports().await;

    progress.phase(OperationPhase::Verifying, "正在确认每个工具的结果…");
    verify::verify_batch(
        &current,
        execution.overall,
        execution.details,
        &execution.per_tool,
    )
    .await
}
