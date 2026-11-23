use crate::infra::paths;
use crate::model::Command;
use std::path::Path;

pub(crate) const PKEXEC_PATH: &str = "/usr/bin/pkexec";

pub(crate) fn has_pkexec() -> bool {
    paths::is_executable(Path::new(PKEXEC_PATH))
}

pub(crate) fn pkexec_issue() -> Option<String> {
    if has_pkexec() {
        None
    } else {
        Some("缺少系统授权组件 pkexec，请先由系统管理员安装。".into())
    }
}

pub(crate) fn is_writable(path: &Path) -> bool {
    let mut current = path;
    while !current.exists() {
        let Some(parent) = current.parent() else {
            return false;
        };
        current = parent;
    }
    #[cfg(unix)]
    {
        rustix::fs::access(current, rustix::fs::Access::WRITE_OK).is_ok()
    }
    #[cfg(not(unix))]
    {
        current
            .metadata()
            .is_ok_and(|metadata| !metadata.permissions().readonly())
    }
}

/// Builds positional-parameter `/bin/sh -c` arguments for executing consecutive
/// privileged steps under a single `pkexec` authorization without shell interpolation.
pub(crate) fn pkexec_batch_arguments(
    commands: &[Command],
    marker: &str,
    offset: usize,
) -> Vec<String> {
    let mut script = String::from("export LC_ALL=C LANG=C\n");
    let mut parameters = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        script.push_str(&format!(
            "printf '%s\\n' '{marker}{}'\n",
            offset + index + 1
        ));
        for argument in std::iter::once(&command.program).chain(&command.arguments) {
            parameters.push(argument.clone());
            script.push_str(&format!("\"${{{}}}\" ", parameters.len()));
        }
        script.push_str("|| exit \"$?\"\n");
        script.push_str(&format!(
            "printf '%s\\n' '{marker}done{}'\n",
            offset + index + 1
        ));
    }
    let mut arguments = vec!["/bin/sh".into(), "-c".into(), script, "tenonry".into()];
    arguments.extend(parameters);
    arguments
}
