use crate::error::{Error, Result};
use crate::infra::process;
use std::path::Path;

pub(super) fn supports_tool(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "git"
            | "git-lfs"
            | "gh"
            | "pnpm"
            | "node"
            | "python"
            | "rust"
            | "go"
            | "java"
            | "dotnet"
            | "cmake"
    )
}

pub(super) fn matches_tool(tool_id: &str, package: &str) -> bool {
    match tool_id {
        "git" => package == "Git.Git",
        "git-lfs" => package == "GitHub.GitLFS",
        "gh" => package == "GitHub.cli",
        "pnpm" => package == "pnpm.pnpm",
        "node" => matches!(package, "OpenJS.NodeJS" | "OpenJS.NodeJS.LTS"),
        "python" => package.starts_with("Python.Python."),
        "rust" => package.starts_with("Rustlang.Rust"),
        "go" => package == "GoLang.Go",
        "java" => package.starts_with("EclipseAdoptium.Temurin."),
        "dotnet" => package.starts_with("Microsoft.DotNet.SDK."),
        "cmake" => package == "Kitware.CMake",
        _ => false,
    }
}

pub(super) async fn default_packages(winget: &Path, tool_id: &str) -> Result<Vec<String>> {
    let id = match tool_id {
        "pnpm" => "pnpm.pnpm",
        "git" => "Git.Git",
        "git-lfs" => "GitHub.GitLFS",
        "gh" => "GitHub.cli",
        "node" => "OpenJS.NodeJS.LTS",
        "rust" => "Rustlang.Rust.MSVC",
        "go" => "GoLang.Go",
        "cmake" => "Kitware.CMake",
        "java" => {
            return Ok(vec![
                newest_id(winget, "EclipseAdoptium.Temurin.", ".JDK").await?,
            ]);
        }
        "dotnet" => return Ok(vec![newest_id(winget, "Microsoft.DotNet.SDK.", "").await?]),
        "python" => return Ok(vec![newest_id(winget, "Python.Python.3.", "").await?]),
        _ => return Err(Error::unsupported("WinGet 暂不支持此工具。")),
    };
    Ok(vec![id.into()])
}

/// Finds the newest numeric WinGet package ID matching `<prefix><major><suffix>`.
async fn newest_id(winget: &Path, prefix: &str, suffix: &str) -> Result<String> {
    let output = process::run(
        winget,
        &process::args(&[
            "search",
            "--id",
            prefix,
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]),
        false,
    )
    .await?
    .checked()?;
    output
        .split_whitespace()
        .filter_map(|word| {
            let rest = word.strip_prefix(prefix)?;
            let major = if suffix.is_empty() {
                rest
            } else {
                rest.strip_suffix(suffix)?
            };
            Some((major.parse::<u32>().ok()?, word.to_owned()))
        })
        .max_by_key(|(major, _)| *major)
        .map(|(_, id)| id)
        .ok_or_else(|| Error::not_found(format!("WinGet 没有返回可安装的 {prefix} 版本。")))
}

pub(super) async fn resolve_node_id(winget: &Path) -> Result<&'static str> {
    for id in ["OpenJS.NodeJS.LTS", "OpenJS.NodeJS"] {
        let output = process::run(
            winget,
            &process::args(&[
                "list",
                "--id",
                id,
                "--exact",
                "--source",
                "winget",
                "--accept-source-agreements",
                "--disable-interactivity",
            ]),
            false,
        )
        .await?;
        if output.status.success()
            && output
                .text
                .lines()
                .any(|line| line.split_whitespace().any(|word| word == id))
        {
            return Ok(id);
        }
        // APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND is an ordinary miss.
        if !output.status.success()
            && output.status.code().map(|code| code as u32) != Some(0x8A150014)
        {
            return Err(output.checked().unwrap_err());
        }
    }
    Err(Error::conflict(
        "已安装 Node.js，但 WinGet 无法确认它的包标识。请先通过 Windows 设置处理。",
    ))
}

pub(super) async fn verify_package(winget: &Path, id: &str) -> Result<()> {
    let output = process::run(
        winget,
        &process::args(&[
            "list",
            "--id",
            id,
            "--exact",
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]),
        false,
    )
    .await?
    .checked_stdout()?;
    if output
        .lines()
        .any(|line| line.split_whitespace().any(|word| word == id))
    {
        Ok(())
    } else {
        Err(Error::conflict(format!(
            "WinGet 无法精确识别 {id}，请通过 Windows 设置处理此安装。"
        )))
    }
}
