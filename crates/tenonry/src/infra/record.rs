use crate::error::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, File};
use std::io::{ErrorKind as IoErrorKind, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub(crate) fn load<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<Option<T>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == IoErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::persistence(format!(
                "无法读取环境记录 {}：{error}",
                path.display()
            )));
        }
    };
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::persistence(error.to_string()))?;
    if bytes.len() as u64 > limit {
        return Err(Error::persistence("环境记录过大，原文件已保留。"));
    }
    serde_json::from_slice(&bytes).map(Some).map_err(|_| {
        Error::persistence(format!(
            "环境记录 {} 无法解析，原文件已保留。",
            path.display()
        ))
    })
}

pub(crate) fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| Error::persistence("环境记录路径无效。"))?;
    fs::create_dir_all(directory).map_err(|error| Error::persistence(error.to_string()))?;
    let bytes = serde_json::to_vec(value).map_err(|error| Error::persistence(error.to_string()))?;
    let mut file =
        NamedTempFile::new_in(directory).map_err(|error| Error::persistence(error.to_string()))?;
    file.write_all(&bytes)
        .and_then(|()| file.as_file().sync_all())
        .map_err(|error| Error::persistence(format!("无法写入环境记录：{error}")))?;
    file.persist(path)
        .map_err(|error| Error::persistence(format!("无法保存环境记录：{error}")))?;
    #[cfg(unix)]
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| Error::persistence(format!("无法同步环境记录：{error}")))?;
    Ok(())
}
