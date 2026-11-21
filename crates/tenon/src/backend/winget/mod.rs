mod inventory;
mod mapping;
mod preflight;

use crate::backend::{BackendContext, BackendDriver, CandidateTargets, common};
use crate::error::{Error, Result};
use crate::infra::{paths, process};
use crate::model::{
    Action, BackendId, Command, Package, Plan, Preview, PreviewState, ToolReport, ToolSpec,
    TransactionPlan,
};
use inventory::RegistryInventory;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct WingetBackend {
    path: PathBuf,
    installed: Vec<Package>,
    inventory: RegistryInventory,
    catalog_basis: Option<String>,
}

impl WingetBackend {
    pub(crate) async fn detect() -> Result<Option<Self>> {
        if !cfg!(windows) {
            return Ok(None);
        }
        let Some(path) = paths::find("winget") else {
            return Ok(None);
        };
        let inventory = inventory::collect_inventory().await?;
        let mut installed: Vec<_> = inventory
            .applications
            .iter()
            .filter_map(|app| app.package())
            .collect();
        installed.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        Ok(Some(Self {
            path,
            installed,
            inventory,
            catalog_basis: None,
        }))
    }

    async fn resolve_existing_packages(
        &self,
        action: Action,
        packages: Vec<String>,
    ) -> Result<Vec<String>> {
        if action == Action::Install {
            return Ok(packages);
        }
        let mut resolved = Vec::with_capacity(packages.len());
        for package in packages {
            if package == "OpenJS.NodeJS" {
                resolved.push(mapping::resolve_node_id(&self.path).await?.into());
            } else {
                mapping::verify_package(&self.path, &package).await?;
                resolved.push(package);
            }
        }
        Ok(resolved)
    }

    fn build_commands(
        &self,
        action: Action,
        packages: &[String],
        preview: &Preview,
    ) -> Result<Vec<Command>> {
        common::validate_package_names(packages)?;
        if preview.state == PreviewState::Current {
            return Ok(Vec::new());
        }
        let program: String = self.path.to_string_lossy().into();
        let verb = match action {
            Action::Install => "install",
            Action::Update => "upgrade",
            Action::Uninstall => "uninstall",
        };
        let mut commands = Vec::with_capacity(packages.len());
        for package in packages {
            let target = preview
                .targets
                .iter()
                .find(|target| target.name == *package)
                .ok_or_else(|| Error::internal("缺少目标版本。"))?;
            if action == Action::Update && target.installed == target.candidate {
                continue;
            }
            let mut arguments = process::args(&[
                verb,
                "--id",
                package,
                "--exact",
                "--silent",
                "--disable-interactivity",
                "--accept-source-agreements",
            ]);
            if action == Action::Uninstall {
                arguments.push("--all-versions".into());
            } else {
                arguments.extend(process::args(&[
                    "--source",
                    "winget",
                    "--accept-package-agreements",
                ]));
                if action == Action::Update {
                    arguments.push("--uninstall-previous".into());
                }
                let candidate = target
                    .candidate
                    .clone()
                    .ok_or_else(|| Error::internal("缺少目标版本。"))?;
                arguments.extend(["--version".into(), candidate]);
            }
            commands.push(Command {
                program: program.clone(),
                arguments,
                administrator: false,
            });
        }
        Ok(commands)
    }
    pub(crate) fn supports_tool_id(tool_id: &str) -> bool {
        mapping::supports_tool(tool_id)
    }

    pub(crate) fn is_benign_exit_code(action: Action, status: std::process::ExitStatus) -> bool {
        // WinGet returns 0x8A15002B when no applicable upgrade is available.
        action == Action::Update && status.code().map(|code| code as u32) == Some(0x8A15002B)
    }

    pub(crate) async fn check_busy() -> Result<bool> {
        #[cfg(windows)]
        {
            let output = process::run(
                Path::new("tasklist.exe"),
                &process::args(&["/FO", "CSV", "/NH"]),
                false,
            )
            .await?
            .checked_stdout()?;
            Ok(output
                .to_ascii_lowercase()
                .lines()
                .any(|line| line.contains("winget.exe") || line.contains("msiexec.exe")))
        }
        #[cfg(not(windows))]
        {
            Ok(false)
        }
    }
}

impl BackendDriver for WingetBackend {
    fn id(&self) -> BackendId {
        BackendId::Winget
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

    async fn owned_files(&self, spec: &ToolSpec, _packages: &[Package]) -> Result<Vec<PathBuf>> {
        Ok(self.inventory.owned_files(spec.id))
    }

    fn set_catalog_basis(&mut self, basis: Option<String>) {
        self.catalog_basis = basis;
    }

    fn catalog_basis(&self) -> Option<&str> {
        self.catalog_basis.as_deref()
    }

    async fn refresh_catalog(&self, _directory: &Path) -> Result<String> {
        process::metadata(
            &self.path,
            &process::args(&[
                "source",
                "update",
                "--name",
                "winget",
                "--disable-interactivity",
            ]),
        )
        .await?
        .checked()?;
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
            Action::Install => mapping::default_packages(&self.path, spec.id).await?,
            Action::Update => self.update_packages(spec, &tool.packages),
            Action::Uninstall => tool
                .packages
                .iter()
                .map(|pkg| pkg.name.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        };
        let packages = self.resolve_existing_packages(action, packages).await?;
        let preview = if action == Action::Uninstall {
            let targets: Vec<_> = packages
                .iter()
                .map(|name| crate::model::Change {
                    name: name.clone(),
                    installed: self.installed_version_for(name),
                    candidate: None,
                    dependency: false,
                })
                .collect();
            Preview {
                state: PreviewState::Available,
                basis: "本机安装记录".into(),
                changes: targets.clone(),
                targets,
                details: String::new(),
            }
        } else {
            preflight::prepare(self, action, &packages).await?
        };
        let commands = self.build_commands(action, &packages, &preview)?;
        let impact = match (action, spec.id) {
            (Action::Uninstall, _) => "移除该应用的所有已安装版本。",
            (Action::Update, "python") => "保持 Python 当前版本系列。",
            _ => "",
        }
        .into();
        Ok(Plan {
            tool,
            action,
            backend: BackendId::Winget,
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
        let packages = self
            .resolve_existing_packages(Action::Update, packages)
            .await?;
        let preview = preflight::prepare(self, Action::Update, &packages).await?;
        let commands = self.build_commands(Action::Update, &packages, &preview)?;
        Ok(TransactionPlan {
            backend: BackendId::Winget,
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
        Self::package_names_match(expected, &actual.name)
    }
}

impl WingetBackend {
    pub(super) fn package_names_match(left: &str, right: &str) -> bool {
        left == right
            || (matches!(left, "OpenJS.NodeJS" | "OpenJS.NodeJS.LTS")
                && matches!(right, "OpenJS.NodeJS" | "OpenJS.NodeJS.LTS"))
    }

    pub(super) fn installed_version_for(&self, name: &str) -> Option<String> {
        self.installed
            .iter()
            .find(|package| Self::package_names_match(name, &package.name))
            .map(|package| package.version.clone())
    }
}
