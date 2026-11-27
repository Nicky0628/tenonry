mod mapping;
mod metadata;
mod preflight;

use crate::backend::{BackendContext, BackendDriver, CandidateTargets};
use crate::error::{Error, Result};
use crate::infra::{paths, process};
use crate::model::{
    Action, BackendId, Command, Package, Plan, ToolReport, ToolSpec, TransactionPlan, VersionList,
};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(super) struct NpmInstallation {
    pub(super) node: PathBuf,
    pub(super) cli: PathBuf,
    pub(super) prefix: PathBuf,
    pub(super) root: PathBuf,
    pub(super) registry: String,
    global_config: PathBuf,
}

#[derive(Debug, Clone)]
pub(super) struct NpmCommandError {
    pub(super) code: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) detail: Option<String>,
    pub(super) raw: String,
}

impl NpmCommandError {
    pub(super) fn is_code(&self, expected: &str) -> bool {
        self.code
            .as_deref()
            .is_some_and(|code| code.eq_ignore_ascii_case(expected))
    }

    pub(super) fn formatted_detail(&self) -> String {
        let mut parts = Vec::new();
        let code = self
            .code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let summary = self
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let detail = self
            .detail
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        match (code, summary) {
            (Some(code), Some(summary)) => parts.push(format!("npm {code}: {summary}")),
            (Some(code), None) => parts.push(format!("npm {code}")),
            (None, Some(summary)) => parts.push(summary.to_owned()),
            (None, None) => {}
        }
        if let Some(detail) = detail
            && summary != Some(detail)
        {
            parts.push(detail.to_owned());
        }
        if parts.is_empty() {
            self.raw.clone()
        } else {
            parts.join("\n")
        }
    }

    pub(super) fn into_error(self, fallback_prefix: &str) -> Error {
        let detail = self.formatted_detail();
        let code = self.code.as_deref().unwrap_or("");
        let summary = self.summary.as_deref().unwrap_or("");

        if code.eq_ignore_ascii_case("EBADENGINE") || code.eq_ignore_ascii_case("EBADPLATFORM") {
            return Error::conflict(format!(
                "所选版本的安装条件未通过，请检查 Node.js 版本及平台支持。\n{detail}"
            ));
        }
        if code.eq_ignore_ascii_case("ETARGET") || code.eq_ignore_ascii_case("E404") {
            return Error::not_found(format!(
                "npm 软件源中未找到目标软件包或版本，请刷新后重试。\n{detail}"
            ));
        }
        if matches!(code.to_ascii_uppercase().as_str(), "E401" | "E403" | "EOTP") {
            return Error::authorization(format!(
                "npm 软件源拒绝访问或需要认证，请检查软件源与登录配置。\n{detail}"
            ));
        }
        if code.eq_ignore_ascii_case("EACCES") || code.eq_ignore_ascii_case("EPERM") {
            return Error::authorization(format!(
                "npm 缺少所需目录权限，请检查缓存或全局目录权限。\n{detail}"
            ));
        }
        if code.eq_ignore_ascii_case("ERESOLVE") {
            return Error::conflict(format!("npm 依赖解析冲突，无法完成操作。\n{detail}"));
        }
        if matches!(
            code.to_ascii_uppercase().as_str(),
            "ENOTFOUND"
                | "ETIMEDOUT"
                | "ECONNREFUSED"
                | "ECONNRESET"
                | "EAI_AGAIN"
                | "EHOSTUNREACH"
                | "ENETUNREACH"
                | "ESOCKETTIMEDOUT"
                | "ERR_SOCKET_TIMEOUT"
                | "FETCH_ERROR"
        ) || summary.contains("FetchError")
        {
            return Error::command_failed(format!(
                "无法连接 npm 软件源，请检查网络、代理或软件源配置。\n{detail}"
            ));
        }

        Error::command_failed(format!("{fallback_prefix}\n{detail}"))
    }
}

pub(super) enum NpmJsonFailure {
    Npm(NpmCommandError),
    Other(Error),
}

impl NpmJsonFailure {
    pub(super) fn into_error(self, fallback_prefix: &str) -> Error {
        match self {
            Self::Npm(failure) => failure.into_error(fallback_prefix),
            Self::Other(error) => error,
        }
    }
}

fn parse_json_object(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    serde_json::from_str::<Value>(trimmed)
        .ok()
        .filter(Value::is_object)
        .or_else(|| {
            // npm 11 prints summary lines (e.g. `add pnpm 10.33.0`) before the JSON object.
            trimmed.match_indices('\n').find_map(|(index, _)| {
                let tail = trimmed[index + 1..].trim_start();
                tail.starts_with('{')
                    .then(|| serde_json::from_str::<Value>(tail).ok())
                    .flatten()
                    .filter(Value::is_object)
            })
        })
}

fn extract_stderr_npm_code(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line
            .trim_start()
            .strip_prefix("npm ERR! code ")
            .or_else(|| line.trim_start().strip_prefix("npm error code "))?;
        let code = rest.split_whitespace().next()?;
        (!code.is_empty()).then(|| code.to_owned())
    })
}

fn extract_npm_error(parsed: Option<&Value>, raw: String) -> NpmCommandError {
    if let Some(error) = parsed.and_then(|value| value.get("error")) {
        if let Some(object) = error.as_object() {
            return NpmCommandError {
                code: object
                    .get("code")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| extract_stderr_npm_code(&raw)),
                summary: object
                    .get("summary")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                detail: object
                    .get("detail")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                raw,
            };
        }
        if let Some(summary) = error.as_str() {
            return NpmCommandError {
                code: extract_stderr_npm_code(&raw),
                summary: Some(summary.to_owned()),
                detail: None,
                raw,
            };
        }
    }
    NpmCommandError {
        code: extract_stderr_npm_code(&raw),
        summary: None,
        detail: None,
        raw,
    }
}

impl NpmInstallation {
    fn base_arguments(&self, args: &[String]) -> Vec<String> {
        let mut result = vec![self.cli.to_string_lossy().into_owned()];
        result.extend_from_slice(args);
        result.extend([
            format!("--prefix={}", self.prefix.display()),
            format!("--registry={}", self.registry),
            "--no-audit".into(),
            "--no-fund".into(),
            "--update-notifier=false".into(),
        ]);
        result
    }

    /// Arguments for bounded read-only probes (`npm view`, `--dry-run`).
    pub(super) fn probe_arguments(&self, args: &[String]) -> Vec<String> {
        let mut result = self.base_arguments(args);
        result.extend(["--fetch-retries=0".into(), "--fetch-timeout=12000".into()]);
        result
    }

    /// Arguments for real install/uninstall execution (preserves npm's retry/timeout resilience).
    pub(super) fn execution_arguments(&self, args: &[String]) -> Vec<String> {
        self.base_arguments(args)
    }

    pub(super) async fn json(&self, args: &[String]) -> Result<Value> {
        let output = process::run(&self.node, &self.probe_arguments(args), false).await?;
        let parsed = parse_json_object(&output.stdout);
        if !output.status.success() {
            let raw = output
                .checked()
                .err()
                .map_or_else(String::new, |error| error.message);
            return Err(
                extract_npm_error(parsed.as_ref(), raw).into_error("读取 npm 软件包信息失败。")
            );
        }
        let parsed = parsed
            .ok_or_else(|| Error::internal("npm 未返回完整的 JSON 信息，请检查 npm 后重试。"))?;
        if parsed.get("error").is_some() {
            return Err(extract_npm_error(Some(&parsed), output.text)
                .into_error("读取 npm 软件包信息失败。"));
        }
        Ok(parsed)
    }

    pub(super) async fn dry_run_json(
        &self,
        args: &[String],
    ) -> std::result::Result<Value, NpmJsonFailure> {
        // npm 9 cannot dry-run a global install before prefix/lib exists. Probe an
        // empty temporary prefix instead of creating directories in the real one.
        let temporary = if self.root.try_exists().map_err(|error| {
            NpmJsonFailure::Other(Error::persistence(format!(
                "无法检查 npm 全局目录：{error}"
            )))
        })? {
            None
        } else {
            Some(tempfile::tempdir().map_err(|error| {
                NpmJsonFailure::Other(Error::persistence(format!(
                    "无法准备 npm 预检目录：{error}"
                )))
            })?)
        };
        let mut installation = self.clone();
        if let Some(directory) = &temporary {
            installation.prefix = directory.path().to_path_buf();
            installation.root = if cfg!(windows) {
                installation.prefix.join("node_modules")
            } else {
                installation.prefix.join("lib/node_modules")
            };
            std::fs::create_dir_all(&installation.root).map_err(|error| {
                NpmJsonFailure::Other(Error::persistence(format!(
                    "无法准备 npm 预检目录：{error}"
                )))
            })?;
        }
        let mut arguments = installation.probe_arguments(args);
        if temporary.is_some() {
            // Changing prefix also changes npm's default global npmrc path. Keep
            // the real configuration (proxy, CA, authentication) for the preview.
            arguments.push(format!("--globalconfig={}", self.global_config.display()));
        }
        let output = process::run(&installation.node, &arguments, false)
            .await
            .map_err(NpmJsonFailure::Other)?;
        let parsed = parse_json_object(&output.stdout);
        if !output.status.success() {
            let raw = output
                .checked()
                .err()
                .map_or_else(String::new, |error| error.message);
            return Err(NpmJsonFailure::Npm(extract_npm_error(parsed.as_ref(), raw)));
        }
        let parsed = parsed.ok_or_else(|| {
            NpmJsonFailure::Other(Error::internal(
                "npm 未返回完整的 JSON 信息，请检查 npm 后重试。",
            ))
        })?;
        if parsed.get("error").is_some() {
            return Err(NpmJsonFailure::Npm(extract_npm_error(
                Some(&parsed),
                output.text,
            )));
        }
        Ok(parsed)
    }

    pub(super) fn bin_directory(&self) -> PathBuf {
        if cfg!(windows) {
            self.prefix.clone()
        } else {
            self.prefix.join("bin")
        }
    }

    fn commands_match(&self, commands: &[Command]) -> bool {
        commands.iter().all(|command| {
            command.program == self.node.to_string_lossy()
                && command
                    .arguments
                    .first()
                    .is_some_and(|argument| argument == &self.cli.to_string_lossy())
                && command
                    .arguments
                    .contains(&format!("--prefix={}", self.prefix.display()))
                && command
                    .arguments
                    .contains(&format!("--registry={}", self.registry))
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NpmBackend {
    pub(super) installation: NpmInstallation,
    pub(super) installed: Vec<Package>,
}

impl NpmBackend {
    pub(crate) async fn detect() -> Result<Option<Self>> {
        let Some(node) = paths::find("node") else {
            return Ok(None);
        };
        let Some(npm) = paths::find(if cfg!(windows) { "npm.cmd" } else { "npm" }) else {
            return Ok(None);
        };
        let cli = if cfg!(windows) {
            npm.parent()
                .ok_or_else(|| Error::internal("无法确定 npm 位置。"))?
                .join("node_modules/npm/bin/npm-cli.js")
        } else {
            paths::resolved(&npm)
        };
        if !cli.is_file() || cli.file_name().is_none_or(|name| name != "npm-cli.js") {
            return Err(Error::conflict(
                "当前 npm 入口不是可识别的 npm 安装，请检查 Node.js 与 npm。",
            ));
        }

        let mut read_args = vec![cli.to_string_lossy().into_owned()];
        read_args.extend(process::args(&[
            "config",
            "get",
            "prefix",
            "registry",
            "globalconfig",
            "--global",
            "--json",
        ]));
        let output = process::run(&node, &read_args, false)
            .await?
            .checked_stdout()?;

        // npm 9 prints key=value for multi-key `config get` even with `--json`.
        let config: Value = match serde_json::from_str::<Value>(&output) {
            Ok(value) if value.is_object() => value,
            _ => {
                let mut values = serde_json::Map::new();
                for line in output.lines().filter(|line| !line.trim().is_empty()) {
                    let (key, value) = line
                        .split_once('=')
                        .ok_or_else(|| Error::internal("无法解析 npm 配置。"))?;
                    let key = key.trim();
                    if !matches!(key, "prefix" | "registry" | "globalconfig")
                        || values.contains_key(key)
                    {
                        return Err(Error::internal("npm 配置字段无效。"));
                    }
                    values.insert(key.into(), Value::String(value.trim().into()));
                }
                Value::Object(values)
            }
        };

        let prefix = PathBuf::from(
            config["prefix"]
                .as_str()
                .ok_or_else(|| Error::internal("npm 未提供全局目录。"))?,
        );
        let registry: String = config["registry"]
            .as_str()
            .ok_or_else(|| Error::internal("npm 未提供软件源。"))?
            .into();
        let parsed =
            url::Url::parse(&registry).map_err(|_| Error::conflict("npm 软件源地址无效。"))?;
        if !matches!(parsed.scheme(), "https" | "http")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::conflict(
                "npm 软件源地址包含无法保存的认证或查询信息，请使用标准源地址。",
            ));
        }
        if !prefix.is_absolute() {
            return Err(Error::conflict("npm 全局目录不是绝对路径。"));
        }
        let global_config = PathBuf::from(
            config["globalconfig"]
                .as_str()
                .ok_or_else(|| Error::internal("npm 未提供全局配置路径。"))?,
        );
        if !global_config.is_absolute() {
            return Err(Error::conflict("npm 全局配置不是绝对路径。"));
        }
        let root = if cfg!(windows) {
            prefix.join("node_modules")
        } else {
            prefix.join("lib/node_modules")
        };

        let installation = NpmInstallation {
            node,
            cli,
            prefix,
            root,
            registry,
            global_config,
        };
        let installed = read_global_packages(&installation.root)?;
        Ok(Some(Self {
            installation,
            installed,
        }))
    }
    pub(crate) fn supports_tool_id(tool_id: &str) -> bool {
        mapping::supports_tool(tool_id)
    }

    pub(crate) async fn check_busy() -> Result<bool> {
        #[cfg(target_os = "linux")]
        {
            let entries =
                std::fs::read_dir("/proc").map_err(|error| Error::internal(error.to_string()))?;
            for entry in entries.flatten() {
                if !entry
                    .file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|ch| ch.is_ascii_digit())
                {
                    continue;
                }
                let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) else {
                    continue;
                };
                let npm_command =
                    std::fs::read(entry.path().join("cmdline")).is_ok_and(|command| {
                        command
                            .split(|byte| *byte == 0)
                            .any(|arg| String::from_utf8_lossy(arg).ends_with("/npm-cli.js"))
                    });
                if comm.trim().starts_with("npm") || npm_command {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        #[cfg(windows)]
        {
            let system = std::env::var_os("SystemRoot")
                .ok_or_else(|| Error::internal("无法确定 Windows 系统目录。"))?;
            let powershell =
                PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
            let npm = process::run(
                &powershell,
                &process::args(&[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "$ErrorActionPreference='Stop'; $p=@(Get-CimInstance Win32_Process -Filter \"Name='node.exe'\"); if ($p | Where-Object { -not $_.CommandLine -or $_.CommandLine -match 'npm-cli\\.js' }) { Write-Output 'active' }",
                ]),
                false,
            )
            .await?
            .checked_stdout()?;
            Ok(!npm.trim().is_empty())
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            Ok(false)
        }
    }
}

fn read_global_packages(root: &std::path::Path) -> Result<Vec<Package>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(Error::internal(format!(
                "无法读取 npm 全局目录 {}：{error}",
                root.display()
            )));
        }
    };
    let mut installed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Error::internal(error.to_string()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let meta =
            std::fs::symlink_metadata(&path).map_err(|error| Error::internal(error.to_string()))?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            continue;
        }
        if name.starts_with('@') {
            let scoped =
                std::fs::read_dir(&path).map_err(|error| Error::internal(error.to_string()))?;
            for sub in scoped {
                let sub = sub.map_err(|error| Error::internal(error.to_string()))?;
                let sub_path = sub.path();
                let sub_meta = std::fs::symlink_metadata(&sub_path)
                    .map_err(|error| Error::internal(error.to_string()))?;
                if sub_meta.file_type().is_symlink() || !sub_meta.is_dir() {
                    continue;
                }
                if let Some(pkg) = read_package_manifest(&sub_path)? {
                    installed.push(pkg);
                }
            }
        } else if let Some(pkg) = read_package_manifest(&path)? {
            installed.push(pkg);
        }
    }
    installed.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(installed)
}

fn read_package_manifest(directory: &std::path::Path) -> Result<Option<Package>> {
    let manifest_path = directory.join("package.json");
    let bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::internal(format!(
                "无法读取 {}：{error}",
                manifest_path.display()
            )));
        }
    };
    let manifest: Value =
        serde_json::from_slice(&bytes).map_err(|_| Error::internal("npm 安装列表记录损坏。"))?;
    let name = manifest["name"]
        .as_str()
        .ok_or_else(|| Error::internal("npm 安装列表缺少包名。"))?;
    let version = manifest["version"]
        .as_str()
        .ok_or_else(|| Error::internal("npm 安装列表缺少版本信息。"))?;
    Ok(Some(Package {
        name: name.into(),
        version: version.into(),
    }))
}

impl BackendDriver for NpmBackend {
    fn id(&self) -> BackendId {
        BackendId::Npm
    }

    fn installed_packages_all(&self) -> &[Package] {
        &self.installed
    }

    fn supports_tool(&self, spec: &ToolSpec) -> bool {
        mapping::supports_tool(spec.id)
    }

    fn installed_packages(&self, spec: &ToolSpec) -> Vec<Package> {
        let Some(target) = mapping::package_name(spec.id) else {
            return Vec::new();
        };
        self.installed
            .iter()
            .filter(|package| package.name == target)
            .cloned()
            .collect()
    }

    async fn owned_files(&self, spec: &ToolSpec, packages: &[Package]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for package in packages {
            let directory = self.installation.root.join(&package.name);
            let text = std::fs::read(directory.join("package.json")).map_err(|error| {
                Error::internal(format!("无法读取 {} 的安装记录：{error}", package.name))
            })?;
            let manifest: Value =
                serde_json::from_slice(&text).map_err(|_| Error::internal("npm 安装记录损坏。"))?;
            if manifest["name"] != package.name || manifest["version"] != package.version {
                return Err(Error::plan_expired("npm 安装状态已变化，请刷新后重试。"));
            }
            for program in spec.programs {
                let target = manifest["bin"]
                    .get(*program)
                    .and_then(Value::as_str)
                    .or_else(|| manifest["bin"].as_str());
                if let Some(target) = target {
                    let resolved_target = paths::resolved(&directory.join(target));
                    if !resolved_target.starts_with(paths::resolved(&directory)) {
                        return Err(Error::conflict(
                            "npm 命令入口超出软件包目录，无法确认归属。",
                        ));
                    }
                    let bin = self.installation.bin_directory().join(if cfg!(windows) {
                        format!("{program}.cmd")
                    } else {
                        (*program).into()
                    });
                    if cfg!(windows) {
                        let launcher = std::fs::read_to_string(&bin).map_err(|error| {
                            Error::internal(format!("无法读取 npm 命令入口：{error}"))
                        })?;
                        let expected = format!(
                            "node_modules\\{}\\{}",
                            package.name.replace('/', "\\"),
                            target.trim_start_matches("./").replace('/', "\\")
                        );
                        if !launcher.contains(&expected) {
                            return Err(Error::conflict("npm 命令入口与安装记录不符。"));
                        }
                        files.push(bin);
                    } else {
                        if bin.exists() && paths::resolved(&bin) == resolved_target {
                            files.push(bin);
                        }
                        files.push(resolved_target);
                    }
                }
            }
        }
        Ok(files)
    }

    fn catalog_basis(&self) -> Option<&str> {
        Some("npm dist-tags.latest")
    }

    async fn check_updates(
        &self,
        ctx: &BackendContext,
        packages: &[String],
    ) -> Result<CandidateTargets> {
        if packages.is_empty() {
            return Ok(CandidateTargets::new());
        }
        metadata::check_updates(ctx, self, packages).await
    }

    async fn list_versions(
        &self,
        ctx: &BackendContext,
        spec: &ToolSpec,
        _tool: &ToolReport,
    ) -> Result<VersionList> {
        metadata::list_versions(ctx, self, spec).await
    }

    async fn prepare_plan(
        &self,
        ctx: &BackendContext,
        spec: &ToolSpec,
        tool: ToolReport,
        action: Action,
        version: Option<String>,
    ) -> Result<Plan> {
        preflight::prepare(ctx, self, spec, tool, action, version).await
    }

    async fn prepare_batch_transaction(
        &self,
        _ctx: &BackendContext,
        plans: &[Plan],
    ) -> Result<TransactionPlan> {
        let plan = plans
            .first()
            .ok_or_else(|| Error::invalid_input("批量计划为空。"))?;
        Ok(TransactionPlan::from_single(plan))
    }

    async fn validate_destination(&self, transaction: &TransactionPlan) -> Result<()> {
        let current = Self::detect()
            .await?
            .ok_or_else(|| Error::plan_expired("npm 运行环境已变化，请重新检查计划。"))?;
        if !current.installation.commands_match(&transaction.commands) {
            return Err(Error::plan_expired(
                "npm 的运行环境、全局目录或软件源已变化，已停止后续操作，请重新检查。",
            ));
        }
        Ok(())
    }

    fn same_destination(&self, plan: &Plan) -> bool {
        self.installation.commands_match(&plan.commands)
    }

    fn check_dependent_removal(&self, spec: &ToolSpec) -> Result<()> {
        if spec.id == "node"
            && self
                .installed
                .iter()
                .any(|package| !matches!(package.name.as_str(), "npm" | "corepack"))
        {
            return Err(Error::conflict(
                "仍有 npm 全局安装的工具。请先通过 npm 移除这些工具，再卸载 Node.js，以免它们无法使用或失去安装记录。",
            ));
        }
        Ok(())
    }
}
