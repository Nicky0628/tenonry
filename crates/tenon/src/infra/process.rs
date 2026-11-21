use crate::error::{Error, Result};
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

pub(crate) struct Output {
    pub(crate) status: ExitStatus,
    pub(crate) text: String,
    pub(crate) stdout: String,
}

impl Output {
    pub(crate) fn checked_stdout(self) -> Result<String> {
        if self.status.success() {
            Ok(self.stdout)
        } else {
            self.checked()
        }
    }

    pub(crate) fn checked(self) -> Result<String> {
        if self.status.success() {
            Ok(self.text)
        } else {
            let tail: String = self
                .text
                .chars()
                .rev()
                .take(4000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            Err(Error::command_failed(format!(
                "命令未完成（退出码 {}）。\n{tail}",
                self.status
                    .code()
                    .map_or("未知".into(), |code| code.to_string()),
            )))
        }
    }
}

pub(crate) type Observer = Arc<dyn Fn(&str) + Send + Sync>;

async fn read_output(
    mut stream: impl AsyncRead + Unpin,
    observer: Option<Observer>,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::new();
    let mut buffer = vec![0u8; 8192];
    let mut truncated = false;
    let mut pending = Vec::new();
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        if output.len() + count > OUTPUT_LIMIT {
            output.drain(..count.min(output.len()));
            truncated = true;
        }
        output.extend_from_slice(&buffer[..count]);
        if let Some(observer) = &observer {
            for byte in &buffer[..count] {
                if (matches!(byte, b'\n' | b'\r') || pending.len() >= 8192) && !pending.is_empty() {
                    observer(&String::from_utf8_lossy(&pending));
                    pending.clear();
                }
                if !matches!(byte, b'\n' | b'\r') {
                    pending.push(*byte);
                }
            }
        }
    }
    if let Some(observer) = observer
        && !pending.is_empty()
    {
        observer(&String::from_utf8_lossy(&pending));
    }
    Ok((output, truncated))
}

/// Reads are bounded. Mutations have no forced timeout because killing a
/// package manager mid-transaction can corrupt its database.
pub(crate) async fn run(program: &Path, args: &[String], mutation: bool) -> Result<Output> {
    run_with_timeout(program, args, mutation, Duration::from_secs(30), None).await
}

pub(crate) async fn metadata(program: &Path, args: &[String]) -> Result<Output> {
    run_with_timeout(program, args, false, Duration::from_secs(45), None).await
}

pub(crate) async fn run_stream(
    program: &Path,
    args: &[String],
    observer: Observer,
) -> Result<Output> {
    run_with_timeout(program, args, true, Duration::from_secs(30), Some(observer)).await
}

pub(crate) async fn version(program: &Path, args: &[&str]) -> Result<String> {
    let output = run_with_timeout(
        program,
        &self::args(args),
        false,
        Duration::from_secs(5),
        None,
    )
    .await?
    .checked()?;
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::command_failed("命令执行成功，但没有返回版本信息。"))
}

async fn run_with_timeout(
    program: &Path,
    args: &[String],
    mutation: bool,
    timeout: Duration,
    observer: Option<Observer>,
) -> Result<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        // Run outside any project directory so toolchain shims do not trigger downloads.
        .current_dir(std::env::temp_dir())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(!mutation);
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "PAGER",
        "GIT_PAGER",
    ] {
        command.env_remove(key);
    }
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command.spawn().map_err(|error| {
        Error::command_failed(format!("无法启动 {}：{error}", program.display()))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::internal("无法读取命令输出。"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::internal("无法读取命令错误输出。"))?;
    let collect = async {
        let (status, stdout, stderr) = tokio::try_join!(
            child.wait(),
            read_output(stdout, observer.clone()),
            read_output(stderr, observer)
        )
        .map_err(|error| Error::command_failed(format!("无法读取命令执行结果：{error}")))?;
        if !mutation && (stdout.1 || stderr.1) {
            return Err(Error::command_failed(
                "检测输出过长，无法完整确认环境；未执行任何安装操作。",
            ));
        }
        let clean = |bytes: &[u8]| {
            String::from_utf8_lossy(bytes)
                .chars()
                .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
                .collect::<String>()
        };
        let stdout = clean(&stdout.0);
        let text = format!("{}\n{}", stdout, clean(&stderr.0));
        Ok(Output {
            status,
            text: text.trim().into(),
            stdout,
        })
    };
    if mutation {
        collect.await
    } else {
        tokio::time::timeout(timeout, collect).await.map_err(|_| {
            Error::command_failed(format!("检测 {} 超时，请稍后重新检查。", program.display()))
        })?
    }
}

pub(crate) fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}
