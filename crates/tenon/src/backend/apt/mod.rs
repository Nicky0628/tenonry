mod activation;
mod mapping;
mod preflight;
mod protection;
mod versions;

use crate::backend::{BackendContext, BackendDriver, CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::{paths, privilege, process};
use crate::model::{
    Action, BackendId, Command, OperationPhase, Package, Plan, Preview, PreviewState, ToolReport,
    ToolSpec, TransactionPlan, VersionList,
};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct AptBackend {
    pub(super) path: PathBuf,
    pub(super) query: PathBuf,
    pub(super) installed: Vec<Package>,
    pub(super) protected: BTreeSet<String>,
    pub(super) catalog_arguments: Vec<String>,
    pub(super) catalog_basis: Option<String>,
}

impl AptBackend {
    pub(crate) async fn detect() -> Result<Option<Self>> {
        if !cfg!(target_os = "linux") {
            return Ok(None);
        }
        let path = PathBuf::from("/usr/bin/apt-get");
        let query = PathBuf::from("/usr/bin/dpkg-query");
        if !paths::is_executable(&path) || !paths::is_executable(&query) {
            return Ok(None);
        }

        let text = process::run(
            &query,
            &process::args(&[
                "-W",
                "-f=${db:Status-Abbrev}\t${binary:Package}\t${Version}\n",
            ]),
            false,
        )
        .await?
        .checked_stdout()?;

        let mut installed = Vec::new();
        for line in text.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(Error::internal("无法解析系统包状态，请先检查 APT。"));
            }
            if fields[0].ends_with('c') || fields[0].ends_with('n') {
                continue;
            }
            if !fields[0].ends_with('i') {
                return Err(Error::conflict(
                    "系统包存在未完成的安装或配置，请先通过 APT 修复，再管理环境。",
                ));
            }
            let name = fields[1];
            if !common::is_valid_package_name(name) {
                return Err(Error::conflict(
                    "系统包记录包含无法识别的包名，未开放环境修改。",
                ));
            }
            installed.push(Package {
                name: name.into(),
                version: fields[2].into(),
            });
        }
        installed.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        let protected = protection::collect_protected().await?;

        Ok(Some(Self {
            path,
            query,
            installed,
            protected,
            catalog_arguments: Vec::new(),
            catalog_basis: None,
        }))
    }

    pub(super) fn is_protected_package(&self, package: &str) -> bool {
        self.protected.contains(mapping::base_name(package))
    }

    /// Privileged commands must not recreate user-owned pkgcache files as root.
    pub(super) fn execution_catalog_arguments(&self) -> Vec<String> {
        self.catalog_arguments
            .iter()
            .map(|argument| {
                for key in ["Dir::Cache::pkgcache=", "Dir::Cache::srcpkgcache="] {
                    if argument.starts_with(key) {
                        return key.into();
                    }
                }
                argument.clone()
            })
            .collect()
    }

    fn build_commands(
        &self,
        action: Action,
        packages: &[String],
        preview: &Preview,
    ) -> Result<Vec<Command>> {
        common::validate_package_names(packages)?;
        if let Some(issue) = self.authorization_issue() {
            return Err(Error::authorization(issue));
        }
        if preview.state == PreviewState::Current {
            return Ok(Vec::new());
        }
        if action == Action::Uninstall {
            // `dpkg --remove` refuses removal when another installed package depends on it.
            let mut arguments = process::args(&["--remove"]);
            arguments.extend_from_slice(packages);
            return Ok(vec![Command {
                program: "/usr/bin/dpkg".into(),
                arguments,
                administrator: true,
            }]);
        }

        let mut arguments = self.execution_catalog_arguments();
        arguments.extend(process::args(if action == Action::Update {
            &[
                "--yes",
                "--no-remove",
                "--only-upgrade",
                "-o",
                "Dpkg::Options::=--force-confdef",
                "-o",
                "Dpkg::Options::=--force-confold",
                "install",
            ]
        } else {
            &[
                "--yes",
                "--no-remove",
                "-o",
                "Dpkg::Options::=--force-confdef",
                "-o",
                "Dpkg::Options::=--force-confold",
                "install",
            ]
        }));
        for name in packages {
            let target = preview
                .targets
                .iter()
                .find(|target| target.name == *name)
                .ok_or_else(|| Error::internal("缺少目标版本。"))?;
            let candidate = target
                .candidate
                .as_deref()
                .ok_or_else(|| Error::internal("缺少目标版本。"))?;
            arguments.push(format!("{name}={candidate}"));
        }
        Ok(vec![Command {
            program: self.path.to_string_lossy().into(),
            arguments,
            administrator: true,
        }])
    }
    pub(crate) fn supports_tool_id(tool_id: &str) -> bool {
        mapping::supports_tool(tool_id)
    }

    pub(crate) fn parse_progress_line(line: &str) -> Option<(OperationPhase, &'static str)> {
        if line.starts_with("Get:") {
            Some((OperationPhase::Downloading, "正在下载软件包…"))
        } else if line.starts_with("Unpacking ") || line.starts_with("Preparing to unpack ") {
            Some((OperationPhase::Installing, "正在解包安装…"))
        } else if line.starts_with("Setting up ") || line.starts_with("Processing triggers for ") {
            Some((OperationPhase::Configuring, "正在配置软件包…"))
        } else if line.starts_with("Removing ") {
            Some((OperationPhase::Removing, "正在移除软件包…"))
        } else {
            None
        }
    }

    pub(crate) async fn check_busy() -> Result<bool> {
        #[cfg(target_os = "linux")]
        {
            common::linux_proc_has_any(&["apt", "apt-get", "dpkg", "pkexec"])
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(false)
        }
    }
}

impl BackendDriver for AptBackend {
    fn id(&self) -> BackendId {
        BackendId::Apt
    }

    fn installed_packages_all(&self) -> &[Package] {
        &self.installed
    }

    fn supports_tool(&self, spec: &ToolSpec) -> bool {
        mapping::supports_tool(spec.id)
    }

    fn installed_packages(&self, spec: &ToolSpec) -> Vec<Package> {
        self.installed
            .iter()
            .filter(|package| mapping::matches_tool(spec.id, &package.name))
            .cloned()
            .collect()
    }

    async fn owned_files(&self, _spec: &ToolSpec, packages: &[Package]) -> Result<Vec<PathBuf>> {
        if packages.is_empty() {
            return Ok(Vec::new());
        }
        let mut arguments = process::args(&["-L"]);
        let names: BTreeSet<_> = packages.iter().map(|pkg| pkg.name.clone()).collect();
        arguments.extend(names);
        let output = process::run(&self.query, &arguments, false)
            .await?
            .checked_stdout()?;
        Ok(output
            .lines()
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .collect())
    }

    fn is_protected_tool(&self, _spec: &ToolSpec, packages: &[Package]) -> bool {
        packages
            .iter()
            .any(|package| self.is_protected_package(&package.name))
    }

    fn authorization_issue(&self) -> Option<String> {
        privilege::pkexec_issue()
    }

    fn update_packages(&self, spec: &ToolSpec, packages: &[Package]) -> Vec<String> {
        let mut names: BTreeSet<_> = packages.iter().map(|pkg| pkg.name.clone()).collect();
        if spec.id == "python" {
            let concrete: BTreeSet<_> = names
                .iter()
                .filter(|name| name.starts_with("python3.") || name.starts_with("python2."))
                .cloned()
                .collect();
            if !concrete.is_empty() {
                names = concrete;
            }
        }
        names.into_iter().collect()
    }

    fn configure_catalog(&mut self, directory: &Path) -> Result<()> {
        std::fs::create_dir_all(directory.join("lists/partial"))
            .map_err(|error| Error::persistence(error.to_string()))?;
        self.catalog_arguments = vec![
            "-o".into(),
            format!("Dir::State::lists={}", directory.join("lists").display()),
            "-o".into(),
            format!(
                "Dir::Cache::pkgcache={}",
                directory.join("packages.bin").display()
            ),
            "-o".into(),
            format!(
                "Dir::Cache::srcpkgcache={}",
                directory.join("sources.bin").display()
            ),
        ];
        Ok(())
    }

    fn set_catalog_basis(&mut self, basis: Option<String>) {
        self.catalog_basis = basis;
    }

    fn catalog_basis(&self) -> Option<&str> {
        self.catalog_basis.as_deref()
    }

    async fn refresh_catalog(&self, directory: &Path) -> Result<String> {
        let config = directory.join("apt-refresh.conf");
        std::fs::write(
            &config,
            "#clear APT::Update::Pre-Invoke;\n\
             #clear APT::Update::Post-Invoke;\n\
             #clear APT::Update::Post-Invoke-Success;\n\
             Acquire::Languages \"none\";\n\
             Acquire::IndexTargets::deb::DEP-11::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::DEP-11-icons-small::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::DEP-11-icons::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::DEP-11-icons-hidpi::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::DEP-11-icons-large::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::DEP-11-icons-large-hidpi::DefaultEnabled \"false\";\n\
             Acquire::IndexTargets::deb::CNF::DefaultEnabled \"false\";\n",
        )
        .map_err(|error| Error::persistence(error.to_string()))?;

        let mut args = self.catalog_arguments.clone();
        args.extend([
            "-c".into(),
            config.to_string_lossy().into(),
            "update".into(),
            "--error-on=any".into(),
            "-o".into(),
            "Acquire::http::Timeout=8".into(),
            "-o".into(),
            "Acquire::https::Timeout=8".into(),
            "-o".into(),
            "Acquire::Retries=0".into(),
        ]);
        // A complete isolated index may span multiple mirrors and large indexes.
        // Keep per-connection limits while allowing the complete refresh to finish.
        process::metadata_with_timeout(&self.path, &args, std::time::Duration::from_secs(120))
            .await?
            .checked()?;
        Ok("软件源已刷新。".into())
    }

    fn local_catalog_revision(&self) -> Result<String> {
        let directory = Path::new("/var/lib/apt/lists");
        let entries = std::fs::read_dir(directory)
            .map_err(|error| Error::persistence(format!("无法读取本机软件源索引：{error}")))?;
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| Error::persistence(error.to_string()))?;
            let name = entry.file_name();
            if name == "lock" || name == "partial" || name == "auxfiles" {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|error| Error::persistence(error.to_string()))?;
            if metadata.is_file() {
                files.push((
                    name,
                    metadata.len(),
                    metadata
                        .modified()
                        .map_err(|error| Error::persistence(error.to_string()))?,
                ));
            }
        }
        files.sort();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        files.hash(&mut hash);
        Ok(format!("system-{:x}", hash.finish()))
    }

    async fn check_updates(
        &self,
        _ctx: &BackendContext,
        packages: &[String],
    ) -> Result<CandidateTargets> {
        if packages.is_empty() {
            return Ok(CandidateTargets::new());
        }
        preflight::check_updates(self, packages).await
    }

    async fn list_versions(
        &self,
        _ctx: &BackendContext,
        spec: &ToolSpec,
        tool: &ToolReport,
    ) -> Result<VersionList> {
        versions::list_versions(self, spec, tool).await
    }

    async fn prepare_plan(
        &self,
        _ctx: &BackendContext,
        spec: &ToolSpec,
        tool: ToolReport,
        action: Action,
        version: Option<String>,
    ) -> Result<Plan> {
        if let Some(version) = version {
            if action == Action::Uninstall {
                return Err(Error::invalid_input("卸载时不需要选择版本。"));
            }
            return versions::plan_version(self, spec, tool, action, version).await;
        }
        let packages: Vec<String> = match action {
            Action::Install => {
                if spec.id == "dotnet" {
                    if let Some(newest) = versions::newest_primary_package(self, spec).await? {
                        vec![newest]
                    } else {
                        process::args(mapping::default_packages(spec.id))
                    }
                } else {
                    process::args(mapping::default_packages(spec.id))
                }
            }
            Action::Update => self.update_packages(spec, &tool.packages),
            Action::Uninstall => tool
                .packages
                .iter()
                .map(|pkg| pkg.name.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        };
        let preview = if action == Action::Uninstall {
            common::uninstall_preview(&self.installed, &packages)
        } else {
            preflight::prepare(self, action, &packages).await?
        };
        let mut commands = self.build_commands(action, &packages, &preview)?;
        if action == Action::Uninstall {
            commands.extend(activation::cleanup_commands(&tool));
        }
        let impact = if action == Action::Update && spec.id == "python" {
            "保持 Python 当前版本系列。"
        } else {
            ""
        }
        .into();
        Ok(Plan {
            tool,
            action,
            backend: BackendId::Apt,
            packages,
            commands,
            impact,
            preview,
            version: None,
        })
    }

    async fn prepare_batch_transaction(
        &self,
        _ctx: &BackendContext,
        plans: &[Plan],
    ) -> Result<TransactionPlan> {
        let packages: Vec<_> = plans
            .iter()
            .flat_map(|plan| plan.packages.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let preview = preflight::prepare(self, Action::Update, &packages).await?;
        let commands = self.build_commands(Action::Update, &packages, &preview)?;
        Ok(TransactionPlan {
            backend: BackendId::Apt,
            action: Action::Update,
            tool_ids: plans
                .iter()
                .map(|plan| plan.tool.status.id.clone())
                .collect(),
            packages,
            commands,
            preview,
        })
    }

    fn matches_installed_package(&self, expected: &str, actual: &Package) -> bool {
        actual.name == expected || mapping::base_name(&actual.name) == mapping::base_name(expected)
    }
}
