use super::mapping::base_name;
use crate::error::{Error, Result};
use crate::infra::process;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Debian identifies base-system packages via Priority/Essential flags; we
/// compute the transitive dependency closure so concrete runtimes required by
/// the OS (such as the system Python interpreter) are also protected.
pub(super) async fn collect_protected() -> Result<BTreeSet<String>> {
    let output = process::run(
        Path::new("/usr/bin/dpkg-query"),
        &process::args(&[
            "-W",
            "-f=${db:Status-Abbrev}\t${binary:Package}\t${Priority}\t${Essential}\t${Pre-Depends}, ${Depends}\n",
        ]),
        false,
    )
    .await?
    .checked_stdout()?;

    let mut protected = BTreeSet::new();
    let mut dependencies = BTreeMap::new();
    for line in output.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(Error::internal("无法识别系统自带组件，已暂停环境修改。"));
        }
        if !fields[0].trim_end().ends_with('i') {
            continue;
        }
        let package = base_name(fields[1]).to_owned();
        if fields[3] == "yes" || matches!(fields[2], "required" | "important" | "standard") {
            protected.insert(package.clone());
        }
        dependencies.insert(package, fields[4].to_owned());
    }

    let mut pending: Vec<_> = protected.iter().cloned().collect();
    while let Some(package) = pending.pop() {
        if let Some(deps) = dependencies.get(&package) {
            for dep in deps
                .split([',', '|'])
                .filter_map(|value| value.split_whitespace().next())
            {
                let dep = base_name(dep);
                if dependencies.contains_key(dep) && protected.insert(dep.to_owned()) {
                    pending.push(dep.to_owned());
                }
            }
        }
    }
    Ok(protected)
}
