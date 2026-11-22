/// Resolves the user's interactive login shell PATH on Linux before Tauri or any
/// worker threads start, so GUI launches find user-installed toolchains.
pub(crate) fn restore() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "无法准备 shell 环境检测，将使用启动时的 PATH。".to_owned())?;
        let result = runtime.block_on(capture());
        drop(runtime);
        let path = result
            .map_err(|error| format!("{error}已保留启动时的 PATH，部分工具可能无法直接探测。"))?;
        // SAFETY: Called from main before Tauri initializes and after the
        // single-threaded runtime above has been dropped; no other threads exist.
        unsafe {
            std::env::set_var("PATH", path);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn capture() -> Result<std::ffi::OsString, String> {
    use rustix::process::{Pid, Signal, kill_process_group};
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;

    const LIMIT: usize = 128 * 1024;
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    let mut command = tokio::process::Command::new(shell);
    command
        .args([
            "-ilc",
            "command printf '\\000TENON_PATH\\000'; /usr/bin/printenv PATH; command printf '\\000TENON_END\\000'",
        ])
        .env("DISABLE_AUTO_UPDATE", "true")
        .env("HISTFILE", "/dev/null")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .kill_on_drop(true);
    if let Some(home) = std::env::var_os("HOME") {
        command.current_dir(home);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "无法启动用户 shell。".to_owned())?;
    let group = child
        .id()
        .and_then(|id| i32::try_from(id).ok())
        .and_then(Pid::from_raw);
    let stdout = child.stdout.take().ok_or("无法读取 shell 输出。")?;
    let result: Result<Vec<u8>, String> = tokio::time::timeout(Duration::from_secs(3), async {
        let mut bytes = Vec::new();
        stdout
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "无法读取 shell 输出。".to_owned())?;
        if bytes.len() > LIMIT {
            return Err("shell 输出过长。".into());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "无法确认 shell 执行结果。".to_owned())?;
        if !status.success() {
            return Err("用户 shell 未能正常读取 PATH。".into());
        }
        Ok(bytes)
    })
    .await
    .unwrap_or_else(|_| Err("读取 shell PATH 超时（3 秒）。".into()));

    if let Some(group) = group {
        match kill_process_group(group, Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => {}
            Err(_) => return Err("无法结束 shell 环境检测进程。".into()),
        }
    }
    tokio::time::timeout(Duration::from_secs(1), child.wait())
        .await
        .map_err(|_| "回收 shell 环境检测进程超时。".to_owned())?
        .map_err(|_| "无法回收 shell 环境检测进程。".to_owned())?;
    parse_path(&result?)
}

#[cfg(target_os = "linux")]
fn parse_path(bytes: &[u8]) -> Result<std::ffi::OsString, String> {
    use std::os::unix::ffi::OsStringExt;

    const START: &[u8] = b"\0TENON_PATH\0";
    const END: &[u8] = b"\0TENON_END\0";
    let start = bytes
        .windows(START.len())
        .position(|part| part == START)
        .map(|index| index + START.len())
        .ok_or("shell 未返回 PATH。")?;
    let end = bytes[start..]
        .windows(END.len())
        .position(|part| part == END)
        .map(|index| start + index)
        .ok_or("shell 返回的 PATH 不完整。")?;
    let path = bytes[start..end]
        .strip_suffix(b"\n")
        .ok_or("shell 返回的 PATH 不完整。")?;
    if path.is_empty() || path.contains(&0) {
        return Err("shell 返回的 PATH 无效。".into());
    }
    Ok(std::ffi::OsString::from_vec(path.to_vec()))
}
