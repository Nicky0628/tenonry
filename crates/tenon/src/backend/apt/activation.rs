use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{Command, ToolReport};
use std::path::Path;

pub(super) fn install_commands(tool_id: &str, package: &str) -> Result<Vec<Command>> {
    let entries: Vec<(&str, String)> = match tool_id {
        "go" => {
            let series = package
                .strip_prefix("golang-")
                .and_then(|s| s.strip_suffix("-go"))
                .ok_or_else(|| Error::invalid_input("无法识别 Go 版本。"))?;
            ["go", "gofmt"]
                .into_iter()
                .map(|program| (program, format!("/usr/lib/go-{series}/bin/{program}")))
                .collect()
        }
        "gcc" => vec![("gcc", format!("/usr/bin/{package}"))],
        "clang" => vec![("clang", format!("/usr/bin/{package}"))],
        "ruby" => vec![("ruby", format!("/usr/bin/{package}"))],
        "php" => {
            let bin = package.strip_suffix("-cli").unwrap_or(package);
            vec![("php", format!("/usr/bin/{bin}"))]
        }
        "zig" => vec![("zig", format!("/usr/bin/{package}"))],
        "python" => ["python", "python3"]
            .into_iter()
            .map(|program| (program, format!("/usr/bin/{package}")))
            .collect(),
        _ => Vec::new(),
    };

    let mut commands = Vec::new();
    for (program, target) in entries {
        let link = format!("/usr/local/bin/{program}");
        let group = format!("tenon-{program}");
        let expected_alt = format!("/etc/alternatives/{group}");
        // Never overwrite a manually installed executable; only retarget alternatives
        // managed by Tenon.
        if Path::new(&link).symlink_metadata().is_ok()
            && std::fs::read_link(&link).ok().as_deref() != Some(Path::new(&expected_alt))
        {
            return Err(Error::conflict(format!(
                "{link} 已由其他安装占用，请先通过原安装方式处理。"
            )));
        }
        if std::fs::read_link(&link).is_ok() {
            commands.push(Command {
                program: "/usr/bin/update-alternatives".into(),
                arguments: process::args(&["--remove-all", &group]),
                administrator: true,
            });
        }
        for arguments in [
            process::args(&["--install", &link, &group, &target, "50"]),
            process::args(&["--set", &group, &target]),
        ] {
            commands.push(Command {
                program: "/usr/bin/update-alternatives".into(),
                arguments,
                administrator: true,
            });
        }
    }
    Ok(commands)
}

pub(super) fn cleanup_commands(tool: &ToolReport) -> Vec<Command> {
    let programs: &[&str] = match tool.status.id.as_str() {
        "go" => &["go", "gofmt"],
        "gcc" => &["gcc"],
        "clang" => &["clang"],
        "ruby" => &["ruby"],
        "php" => &["php"],
        "zig" => &["zig"],
        "python" => &["python", "python3"],
        _ => &[],
    };
    programs
        .iter()
        .filter_map(|program| {
            let group = format!("tenon-{program}");
            let link = format!("/usr/local/bin/{program}");
            (std::fs::read_link(&link).ok().as_deref()
                == Some(Path::new(&format!("/etc/alternatives/{group}"))))
            .then(|| Command {
                program: "/usr/bin/update-alternatives".into(),
                arguments: process::args(&["--remove-all", &group]),
                administrator: true,
            })
        })
        .collect()
}
