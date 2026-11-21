mod mapping;
mod preflight;

use crate::backend::{BackendContext, BackendDriver, CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::{paths, privilege, process};
use crate::model::{
    Action, BackendId, Command, Package, Plan, Preview, PreviewState, ToolReport, ToolSpec,
    TransactionPlan,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct DnfBackend {
    pub(super) path: PathBuf,
    pub(super) query: PathBuf,
    pub(super) installed: Vec<Package>,
    pub(super) catalog_arguments: Vec<String>,
    pub(super) catalog_basis: Option<String>,
}

impl DnfBackend {
    pub(crate) async fn detect() -> Result<Option<Self>> {
        if !cfg!(target_os = "linux") {
            return Ok(None);
        }
        let path = PathBuf::from("/usr/bin/dnf");
        let query = PathBuf::from("/usr/bin/rpm");
        if !paths::is_executable(&path) || !paths::is_executable(&query) {
            return Ok(None);
        }
        let text = process::run(
            &query,
            &process::args(&[
                "-qa",
                "--qf",
                "%{NAME}\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\n",
            ]),
            false,
        )
        .await?
        .checked_stdout()?;

        let mut installed = Vec::new();
        for line in text.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 2 {
                return Err(Error::internal(
                    "无法完整解析系统包记录，请先检查包管理器状态。",
                ));
            }
            let name = fields[0];
            if !common::is_valid_package_name(name) {
                return Err(Error::conflict(
                    "系统包记录包含无法识别的包名，未开放环境修改。",
                ));
            }
            for version in &fields[1..] {
                installed.push(Package {
                    name: name.into(),
                    version: version.strip_prefix("0:").unwrap_or(version).into(),
                });
            }
        }
        installed.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        Ok(Some(Self {
            path,
            query,
            installed,
            catalog_arguments: Vec::new(),
            catalog_basis: None,
        }))
    }

    pub(super) async fn query(&self, flags: &[&str], names: &[String]) -> Result<String> {
        let mut args = self.catalog_arguments.clone();
        args.extend(process::args(flags));
        args.extend_from_slice(names);
        process::run(&self.path, &args, false)
            .await?
            .checked_stdout()
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
            let mut arguments = process::args(&["-e"]);
            arguments.extend_from_slice(packages);
            return Ok(vec![Command {
                program: self.query.to_string_lossy().into(),
                arguments,
                administrator: true,
            }]);
        }
        let verb = if action == Action::Install {
            "install"
        } else {
            "upgrade"
        };
        let mut arguments = self.catalog_arguments.clone();
        arguments.extend(process::args(&[
            "--assumeyes",
            "--best",
            "--setopt=install_weak_deps=False",
            verb,
        ]));
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
            arguments.push(format!("{name}-{candidate}"));
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

    pub(crate) async fn check_busy() -> Result<bool> {
        #[cfg(target_os = "linux")]
        {
            common::linux_proc_has_any(&["dnf", "dnf5", "rpm", "pkexec"])
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(false)
        }
    }
}

impl BackendDriver for DnfBackend {
    fn id(&self) -> BackendId {
        BackendId::Dnf
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
        let mut arguments = process::args(&["-ql"]);
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

    fn is_protected_tool(&self, spec: &ToolSpec, packages: &[Package]) -> bool {
        spec.id == "python" && !packages.is_empty()
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
        std::fs::create_dir_all(directory)
            .map_err(|error| Error::persistence(error.to_string()))?;
        self.catalog_arguments = vec![
            format!("--setopt=cachedir={}", directory.display()),
            format!("--setopt=logdir={}", directory.display()),
            "--setopt=metadata_expire=never".into(),
        ];
        Ok(())
    }

    fn set_catalog_basis(&mut self, basis: Option<String>) {
        self.catalog_basis = basis;
    }

    fn catalog_basis(&self) -> Option<&str> {
        self.catalog_basis.as_deref()
    }

    async fn refresh_catalog(&self, _directory: &Path) -> Result<String> {
        let mut args = self.catalog_arguments.clone();
        args.extend(process::args(&["--refresh", "makecache"]));
        process::metadata(&self.path, &args).await?.checked()?;
        Ok("软件源已刷新。".into())
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

    async fn prepare_plan(
        &self,
        _ctx: &BackendContext,
        spec: &ToolSpec,
        tool: ToolReport,
        action: Action,
        version: Option<String>,
    ) -> Result<Plan> {
        if version.is_some() {
            return Err(Error::unsupported("此安装方式暂不支持选择版本。"));
        }
        let packages: Vec<String> = match action {
            Action::Install => process::args(mapping::default_packages(spec.id)),
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
        let commands = self.build_commands(action, &packages, &preview)?;
        let impact = if action == Action::Update && spec.id == "python" {
            "保持 Python 当前版本系列。"
        } else {
            ""
        }
        .into();
        Ok(Plan {
            tool,
            action,
            backend: BackendId::Dnf,
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
            backend: BackendId::Dnf,
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
}
