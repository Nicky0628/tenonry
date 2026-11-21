use crate::error::{Error, Result};
use crate::infra::record;
use crate::model::{BatchPlan, Operation};
use crate::registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) const OPERATIONS_FILE: &str = "operations.json";
pub(super) const MAX_ENTRIES: usize = 30;
pub(super) const MAX_STARTED_IDS: usize = 50_000;
const MAX_LOAD_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SAVE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct JournalEntry {
    pub(super) operation: Operation,
    pub(super) plan: BatchPlan,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Journal {
    pub(super) entries: Vec<JournalEntry>,
    pub(super) started_ids: BTreeSet<String>,
}

impl Journal {
    pub(super) fn load(directory: &Path) -> Result<Self> {
        let journal: Self =
            record::load(&directory.join(OPERATIONS_FILE), MAX_LOAD_BYTES)?.unwrap_or_default();
        if journal.entries.len() > MAX_ENTRIES
            || journal.started_ids.len() > MAX_STARTED_IDS
            || journal.entries.iter().any(|entry| {
                uuid::Uuid::parse_str(&entry.operation.id).is_err()
                    || entry.plan.plans.is_empty()
                    || entry.plan.plans.len() > registry::max_tool_count()
            })
        {
            return Err(Error::persistence("环境操作记录无效，原文件已保留。"));
        }
        Ok(journal)
    }

    pub(super) fn save(&self, directory: &Path) -> Result<()> {
        let mut stored = self.clone();
        for entry in &mut stored.entries {
            entry.operation.persistence_issue = None;
            if let Some(outcome) = &mut entry.operation.outcome {
                outcome.report = None;
            }
        }
        while serde_json::to_vec(&stored)
            .map_err(|error| Error::persistence(error.to_string()))?
            .len()
            > MAX_SAVE_BYTES
        {
            if stored.entries.len() <= 1 {
                return Err(Error::persistence("环境操作记录过大，无法保存。"));
            }
            stored.entries.remove(0);
        }
        record::save(&directory.join(OPERATIONS_FILE), &stored)
    }
}
