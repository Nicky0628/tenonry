use crate::backend::Backend;
use crate::engine::tasks::Progress;
use crate::error::Result;
use crate::infra::{privilege, process};
use crate::model::{Action, BackendId, Command, ExecutionState, OperationPhase, TransactionPlan};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) struct BatchExecution {
    pub(crate) overall: ExecutionState,
    pub(crate) details: String,
    pub(crate) per_tool: BTreeMap<String, ExecutionState>,
}

#[derive(Clone)]
struct ScheduledStep {
    backend: BackendId,
    action: Action,
    command: Command,
}

/// Executes backend-scoped transactions sequentially while merging consecutive
/// privileged steps into a single system authorization prompt.
pub(crate) async fn execute_transactions(
    transactions: &[TransactionPlan],
    progress: &Progress,
) -> BatchExecution {
    let mut steps = Vec::new();
    for transaction in transactions {
        for command in &transaction.commands {
            steps.push(ScheduledStep {
                backend: transaction.backend,
                action: transaction.action,
                command: command.clone(),
            });
        }
    }

    let started = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let result = run_steps(&steps, progress, &started, &completed).await;
    let started_count = started.load(Ordering::SeqCst);
    let completed_count = completed.load(Ordering::SeqCst);

    let mut per_tool = BTreeMap::new();
    let mut offset = 0;
    for transaction in transactions {
        let end = offset + transaction.commands.len();
        let state = if transaction.commands.is_empty() {
            ExecutionState::NotNeeded
        } else if end <= completed_count {
            ExecutionState::Succeeded
        } else if offset < started_count {
            ExecutionState::Failed
        } else {
            ExecutionState::NotStarted
        };
        offset = end;
        for tool_id in &transaction.tool_ids {
            per_tool.insert(tool_id.clone(), state);
        }
    }

    let (overall, details) = match result {
        Ok(log) => (ExecutionState::Succeeded, log),
        Err(error) => (ExecutionState::Failed, error.message),
    };

    BatchExecution {
        overall,
        details,
        per_tool,
    }
}

async fn run_steps(
    steps: &[ScheduledStep],
    progress: &Progress,
    started: &Arc<AtomicUsize>,
    completed: &Arc<AtomicUsize>,
) -> Result<String> {
    let mut log = String::new();
    let marker = format!("TENON_STEP_{}_", uuid::Uuid::new_v4().simple());
    let mut offset = 0;
    let total = steps.len();

    for chunk in
        steps.chunk_by(|prev, next| prev.command.administrator && next.command.administrator)
    {
        let first = &chunk[0];
        let administrator = first.command.administrator;
        progress.phase(
            if administrator {
                OperationPhase::Authorizing
            } else {
                OperationPhase::Executing
            },
            if administrator {
                "等待系统授权…"
            } else {
                "正在执行包管理器操作…"
            },
        );

        let (program, arguments) = if administrator {
            let commands: Vec<Command> = chunk.iter().map(|step| step.command.clone()).collect();
            (
                Path::new(privilege::PKEXEC_PATH),
                privilege::pkexec_batch_arguments(&commands, &marker, offset),
            )
        } else {
            (
                Path::new(&first.command.program),
                first.command.arguments.clone(),
            )
        };

        let chunk_backends: Arc<Vec<BackendId>> =
            Arc::new(chunk.iter().map(|step| step.backend).collect());
        let observer = progress.clone();
        let prefix = marker.clone();
        let started_step = started.clone();
        let completed_step = completed.clone();
        let current_backend_index = Arc::new(AtomicUsize::new(0));
        let chunk_offset = offset;

        if !administrator {
            started.store(offset + 1, Ordering::SeqCst);
        }

        let mut outcome = process::run_stream(
            program,
            &arguments,
            Arc::new(move |line| {
                if let Some(step) = line.strip_prefix(&prefix) {
                    if let Some(done) = step
                        .strip_prefix("done")
                        .and_then(|value| value.parse::<usize>().ok())
                    {
                        completed_step.store(done, Ordering::SeqCst);
                    } else if let Ok(step) = step.parse::<usize>() {
                        started_step.store(step, Ordering::SeqCst);
                        let local_index = step.saturating_sub(chunk_offset + 1);
                        current_backend_index.store(local_index, Ordering::SeqCst);
                        observer.phase(
                            OperationPhase::Executing,
                            &format!("正在执行步骤 {step}/{total}"),
                        );
                    }
                } else {
                    let index = current_backend_index.load(Ordering::SeqCst);
                    if let Some(&backend) = chunk_backends.get(index)
                        && let Some((phase, message)) = Backend::parse_progress_for(backend, line)
                    {
                        observer.phase(phase, message);
                    }
                    let text: String = line
                        .chars()
                        .filter(|ch| !ch.is_control() || *ch == '\t')
                        .collect();
                    observer.output(&text);
                }
            }),
        )
        .await?;

        offset += chunk.len();

        let benign = !administrator
            && Backend::is_benign_exit_for(first.backend, first.action, outcome.status);
        outcome.text = outcome
            .text
            .lines()
            .filter(|line| !line.starts_with(&marker))
            .collect::<Vec<_>>()
            .join("\n");
        let output = if benign {
            outcome.text
        } else {
            outcome.checked()?
        };
        completed.store(offset, Ordering::SeqCst);

        let tail: String = output
            .chars()
            .rev()
            .take(4000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        log.push_str(&tail);
        log.push('\n');
    }

    Ok(log)
}
