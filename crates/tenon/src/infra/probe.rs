use crate::model::{Executable, ToolSpec, ToolStatus};

/// Summarizes `PATH` availability from discovered executables without allowing
/// a shadowed installation to appear active.
pub(crate) fn summarize_status(spec: &ToolSpec, executables: &[Executable]) -> ToolStatus {
    let active = |program: &str| {
        executables
            .iter()
            .find(|entry| entry.active_programs.iter().any(|name| name == program))
    };
    let working = |program: &str| active(program).is_some_and(|entry| entry.version.is_some());
    let found = if spec.require_all {
        spec.programs.iter().all(|program| working(program))
    } else {
        spec.programs.iter().any(|program| working(program))
    };
    let version = spec
        .programs
        .iter()
        .filter_map(|program| active(program))
        .find_map(|entry| entry.version.clone());
    let hint = if found {
        None
    } else {
        spec.programs
            .iter()
            .filter_map(|program| active(program))
            .find_map(|entry| entry.issue.clone())
    };
    ToolStatus {
        id: spec.id.into(),
        name: spec.name.into(),
        found,
        version,
        hint,
    }
}
