use crate::error::{Error, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind as IoErrorKind;
use std::path::Path;

/// Cross-process advisory lock backed by a file handle.
pub(crate) struct FileLock(File);

pub(crate) enum SharedLock {
    Acquired(FileLock),
    Missing,
    Contended,
}

impl FileLock {
    pub(crate) fn acquire_exclusive(path: &Path, busy_message: &str) -> Result<Self> {
        let directory = path
            .parent()
            .ok_or_else(|| Error::persistence("锁文件路径无效。"))?;
        std::fs::create_dir_all(directory)
            .map_err(|error| Error::persistence(format!("无法创建锁目录：{error}")))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|error| Error::persistence(format!("无法打开锁文件：{error}")))?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Self(file)),
            Err(error) if is_contended(&error) => Err(Error::busy(busy_message)),
            Err(error) => Err(Error::persistence(format!("无法获取锁：{error}"))),
        }
    }

    pub(crate) fn try_acquire_shared(path: &Path) -> Result<SharedLock> {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == IoErrorKind::NotFound => return Ok(SharedLock::Missing),
            Err(error) => return Err(Error::persistence(format!("无法读取锁文件：{error}"))),
        };
        match FileExt::try_lock_shared(&file) {
            Ok(()) => Ok(SharedLock::Acquired(Self(file))),
            Err(error) if is_contended(&error) => Ok(SharedLock::Contended),
            Err(error) => Err(Error::persistence(format!("无法读取共享锁：{error}"))),
        }
    }

    pub(crate) fn is_held(path: &Path) -> Result<bool> {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == IoErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(Error::persistence(format!("无法读取锁文件：{error}"))),
        };
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                FileExt::unlock(&file).map_err(|error| Error::persistence(error.to_string()))?;
                Ok(false)
            }
            Err(error) if is_contended(&error) => Ok(true),
            Err(error) => Err(Error::persistence(format!("无法确认操作锁状态：{error}"))),
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

fn is_contended(error: &std::io::Error) -> bool {
    error.kind() == IoErrorKind::WouldBlock
        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
}
