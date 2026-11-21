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
pub(crate) struct PacmanBackend {
    pub(super) path: PathBuf,
    pub(super) installed: Vec<Package>,
    pub(super) catalog_basis: Option<String>,
}

impl PacmanBackend {
    pub(crate) async fn detect() -> Result<Option<Self>> {
        if !cfg!(target_os = "linux") {
            return Ok(None);
        }
        let path = PathBuf::from("/usr/bin/pacman");
        if !paths::is_executable(&path) {
            return Ok(None);
        }
        let text = process::run(&path, &process::args(&["-Q"]), false)
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
                    version: (*version).into(),
                });
            }
        }
        installed.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        Ok(Some(Self {
            path,
            installed,
            catalog_basis: None,
        }))
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
        let mut arguments = if action == Action::Uninstall {
            process::args(&["-R", "--noconfirm"])
        } else {
            // Never trigger an implicit distribution-wide upgrade.
            process::args(&["-S", "--needed", "--noconfirm"])
        };
        arguments.extend_from_slice(packages);
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
            common::linux_proc_has_any(&["pacman", "pkexec"])
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(false)
        }
    }
}

impl BackendDriver for PacmanBackend {
    fn id(&self) -> BackendId {
        BackendId::Pacman
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
        let mut arguments = process::args(&["-Qlq"]);
        let names: BTreeSet<_> = packages.iter().map(|pkg| pkg.name.clone()).collect();
        arguments.extend(names);
        let output = process::run(&self.path, &arguments, false)
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

    fn set_catalog_basis(&mut self, basis: Option<String>) {
        self.catalog_basis = basis;
    }

    fn catalog_basis(&self) -> Option<&str> {
        self.catalog_basis.as_deref()
    }

    async fn refresh_catalog(&self, _directory: &Path) -> Result<String> {
        // Refreshing pacman's system database alone creates a partial-upgrade hazard.
        // `checkupdates` queries a separate database safely.
        let helper = Path::new("/usr/bin/checkupdates");
        if !helper.is_file() {
            return Err(Error::not_found(
                "自动获取 Arch 更新信息需要 pacman-contrib 提供的 checkupdates。请先通过系统安装器安装它。",
            ));
        }
        let result = process::metadata(helper, &process::args(&["--nocolor"])).await?;
        if !result.status.success() && result.status.code() != Some(2) {
            return Err(result.checked().unwrap_err());
        }
        if result.status.success() && !result.stdout.trim().is_empty() {
            return Err(Error::conflict(
                "系统有待完成的 Arch 更新。请先通过系统更新器完成整体更新，再更新开发工具。",
            ));
        }
        Ok("Arch 更新已核对。".into())
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
        let impact = match action {
            Action::Install | Action::Update => "使用系统现有软件源数据库。",
            Action::Uninstall => "",
        }
        .into();
        Ok(Plan {
            tool,
            action,
            backend: BackendId::Pacman,
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
            backend: BackendId::Pacman,
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
