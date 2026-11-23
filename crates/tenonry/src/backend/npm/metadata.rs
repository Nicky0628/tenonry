use super::{NpmBackend, mapping};
use crate::backend::{BackendContext, CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::process;
use crate::model::{BackendId, Change, ToolSpec, VersionInfo, VersionList};
use serde_json::Value;
use std::collections::BTreeMap;

async fn read(ctx: &BackendContext, backend: &NpmBackend, package: &str) -> Result<Value> {
    let key = format!(
        "npm|{}|{}|{}|{}|{package}",
        backend.installation.node.display(),
        backend.installation.cli.display(),
        backend.installation.prefix.display(),
        backend.installation.registry
    );
    if let Some(value) = ctx.metadata.get(&key).await {
        return Ok(value);
    }
    let _request = ctx.metadata.request(&key).await;
    if let Some(value) = ctx.metadata.get(&key).await {
        return Ok(value);
    }
    // npm fetches the complete packument even for one field. Share one request
    // between version selection and latest-version checks, including in flight.
    let args = process::args(&["view", package, "versions", "dist-tags", "--json"]);
    let value = backend.installation.json(&args).await?;
    if !value["dist-tags"].is_object()
        || !(value["versions"].is_array() || value["versions"].is_string())
    {
        return Err(Error::internal("npm 未返回完整的版本与发布标签信息。"));
    }
    ctx.metadata.insert(key, value.clone()).await;
    Ok(value)
}

pub(super) fn semver_triplet(version: &str) -> Option<[u64; 3]> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

pub(super) async fn latest(
    ctx: &BackendContext,
    backend: &NpmBackend,
    name: &str,
) -> Result<String> {
    let value = read(ctx, backend, name).await?;
    let version = value["dist-tags"]["latest"]
        .as_str()
        .filter(|version| semver_triplet(version).is_some())
        .ok_or_else(|| Error::not_found("npm 未提供正式推荐版本，请指定目标版本。"))?;
    Ok(version.into())
}

pub(super) async fn list_versions(
    ctx: &BackendContext,
    backend: &NpmBackend,
    spec: &ToolSpec,
) -> Result<VersionList> {
    let name = mapping::package_name(spec.id)
        .ok_or_else(|| Error::unsupported("此工具没有 npm 软件包。"))?;
    let mut data = read(ctx, backend, name).await?;
    let values = match data["versions"].take() {
        Value::Array(values) => values,
        Value::String(value) => vec![Value::String(value)],
        _ => return Err(Error::internal("npm 版本列表无效。")),
    };
    let current = common::installed_version(&backend.installed, name);
    let mut versions = BTreeMap::new();
    for value in values {
        let version = value
            .as_str()
            .ok_or_else(|| Error::internal("npm 版本记录无效。"))?;
        if let Some(key) = semver_triplet(version) {
            versions.insert(
                key,
                VersionInfo {
                    version: format!("{name}={version}"),
                    label: version.into(),
                    current: current.as_deref() == Some(version),
                },
            );
        }
    }
    Ok(VersionList {
        tool_id: spec.id.into(),
        backend: Some(BackendId::Npm),
        versions: versions.into_values().rev().collect(),
        message: String::new(),
    })
}

pub(super) async fn check_updates(
    ctx: &BackendContext,
    backend: &NpmBackend,
    names: &[String],
) -> Result<CandidateTargets> {
    let mut result = CandidateTargets::new();
    for name in names {
        let value = async {
            let current = common::installed_version(&backend.installed, name)
                .ok_or_else(|| Error::plan_expired("npm 安装状态已变化，请刷新。"))?;
            let latest = latest(ctx, backend, name).await?;
            let candidate = if semver_triplet(&latest)
                .zip(semver_triplet(&current))
                .is_some_and(|(a, b)| a < b)
            {
                current.clone()
            } else {
                latest
            };
            Ok(Change {
                name: name.clone(),
                installed: Some(current),
                candidate: Some(candidate),
                dependency: false,
            })
        }
        .await;
        result.insert(name.clone(), value);
    }
    Ok(result)
}
