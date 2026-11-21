mod history;
mod journal;
mod runner;

use crate::error::{Error, Result};
use crate::model::{
    ExecutionState, Operation, OperationPhase, OperationState, Outcome, VerificationState,
};
use journal::{Journal, OPERATIONS_FILE};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

#[derive(Default)]
struct TaskState {
    journal: Journal,
    active: Option<(String, Instant)>,
    saved_at: Option<Instant>,
    loaded: bool,
    disk_stamp: Option<SystemTime>,
}

impl TaskState {
    fn load(&mut self, directory: &Path) -> Result<()> {
        let stamp = match std::fs::metadata(directory.join(OPERATIONS_FILE)) {
            Ok(metadata) => Some(
                metadata
                    .modified()
                    .map_err(|error| Error::persistence(error.to_string()))?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(Error::persistence(error.to_string())),
        };
        if !self.loaded || stamp != self.disk_stamp {
            self.journal = Journal::load(directory)?;
            self.loaded = true;
            self.disk_stamp = stamp;
        }
        Ok(())
    }

    fn save(&mut self, directory: &Path) -> Result<()> {
        self.journal.save(directory)?;
        self.disk_stamp = Some(
            std::fs::metadata(directory.join(OPERATIONS_FILE))
                .and_then(|metadata| metadata.modified())
                .map_err(|error| Error::persistence(error.to_string()))?,
        );
        self.loaded = true;
        for entry in &mut self.journal.entries {
            entry.operation.persistence_issue = None;
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(crate) struct TaskStore(Arc<Mutex<TaskState>>);

#[derive(Clone)]
pub(crate) struct Progress {
    tasks: TaskStore,
    id: String,
    directory: PathBuf,
}

impl Progress {
    fn update(&self, persist: bool, mutate: impl FnOnce(&mut Operation)) {
        let mut state = self
            .tasks
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let elapsed = state
            .active
            .as_ref()
            .map_or(0, |(_, start)| start.elapsed().as_millis() as u64);
        if let Some(entry) = state
            .journal
            .entries
            .iter_mut()
            .find(|entry| entry.operation.id == self.id)
        {
            entry.operation.elapsed_ms = elapsed;
            mutate(&mut entry.operation);
        }
        if persist
            || state
                .saved_at
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(1))
        {
            let error = state.save(&self.directory).err().map(|error| error.message);
            if let Some(entry) = state
                .journal
                .entries
                .iter_mut()
                .find(|entry| entry.operation.id == self.id)
            {
                entry.operation.persistence_issue = error;
            }
            state.saved_at = Some(Instant::now());
        }
    }

    pub(crate) fn phase(&self, phase: OperationPhase, message: &str) {
        self.update(true, |operation| {
            operation.phase = phase;
            operation.message = message.into();
        });
    }

    pub(crate) fn output(&self, line: &str) {
        self.update(false, |operation| {
            operation.log.push_str(line);
            operation.log.push('\n');
            if operation.log.len() > 64 * 1024 {
                let mut start = operation.log.len() - 64 * 1024;
                while !operation.log.is_char_boundary(start) {
                    start += 1;
                }
                operation.log.drain(..start);
            }
        });
    }

    pub(super) fn checkpoint(&self) -> Result<()> {
        self.phase(OperationPhase::Executing, "准备开始操作…");
        let mut state = self
            .tasks
            .0
            .lock()
            .map_err(|_| Error::internal("无法保存任务。"))?;
        if let Some(entry) = state
            .journal
            .entries
            .iter_mut()
            .find(|entry| entry.operation.id == self.id)
            && let Some(error) = entry.operation.persistence_issue.clone()
        {
            entry.operation.phase = OperationPhase::Checking;
            return Err(Error::persistence(error));
        }
        Ok(())
    }

    pub(super) fn finish(&self, result: Result<Outcome>) {
        self.update(true, |operation| {
            let outcome = result.unwrap_or_else(|error| Outcome {
                completed: false,
                message: error.message.clone(),
                details: operation.log.clone(),
                report: None,
                execution: if operation.phase == OperationPhase::Checking {
                    ExecutionState::NotStarted
                } else {
                    ExecutionState::Unknown
                },
                verification: VerificationState::Unknown,
                items: Vec::new(),
                error: Some(error),
            });
            operation.state = if outcome.completed {
                OperationState::Completed
            } else {
                OperationState::NeedsAttention
            };
            operation.phase = OperationPhase::Finished;
            operation.message = outcome.message.clone();
            if !outcome.items.is_empty() {
                operation.tools = outcome.items.clone();
            }
            operation.outcome = Some(outcome);
        });
        if let Ok(mut state) = self.tasks.0.lock() {
            state.active = None;
        }
    }
}

pub(super) fn classify_recovered(operation: &Operation) -> Operation {
    let mut result = operation.clone();
    if result.state == OperationState::Running {
        result.state = OperationState::NeedsAttention;
        result.phase = OperationPhase::Interrupted;
        result.message = "上次操作意外中断，请确认当前状态。不会自动重新安装。".into();
        result.outcome = Some(Outcome {
            completed: false,
            message: result.message.clone(),
            details: result.log.clone(),
            report: None,
            execution: ExecutionState::Unknown,
            verification: VerificationState::Unknown,
            items: Vec::new(),
            error: Some(Error::internal(result.message.clone())),
        });
    }
    result
}
