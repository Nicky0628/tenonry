use crate::error::{Error, Result};
use crate::infra::{paths, process};
use crate::model::Package;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Registry inventory is independent of PATH so a second Python install or an
/// install awaiting shell reload is never mistaken for missing.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct InstalledApplication {
    display_name: String,
    display_version: Option<String>,
    install_location: Option<String>,
    display_icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct VersionedPath {
    version: Option<String>,
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct PythonPath {
    tag: String,
    install_path: Option<String>,
    executable_path: Option<String>,
}

/// Authoritative Windows installer registry snapshot collected in a single query.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct RegistryInventory {
    #[serde(default)]
    pub(super) applications: Vec<InstalledApplication>,
    #[serde(default)]
    node_paths: Vec<VersionedPath>,
    #[serde(default)]
    python_paths: Vec<PythonPath>,
    #[serde(default)]
    git_paths: Vec<VersionedPath>,
    #[serde(default)]
    temurin_paths: Vec<VersionedPath>,
    #[serde(default)]
    go_paths: Vec<String>,
    #[serde(default)]
    cmake_paths: Vec<String>,
    #[serde(default)]
    dotnet_paths: Vec<String>,
}

const INVENTORY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$roots = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKCU:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
)
$apps = @(foreach ($root in $roots) {
  if (Test-Path $root) {
    Get-ItemProperty "$root\*" |
      Where-Object { $_.DisplayName -and -not $_.SystemComponent -and -not $_.ParentKeyName } |
      Select-Object DisplayName,DisplayVersion,InstallLocation,DisplayIcon
  }
})

$nodePaths = @(foreach ($key in @('HKLM:\SOFTWARE\Node.js', 'HKLM:\SOFTWARE\WOW6432Node\Node.js', 'HKCU:\SOFTWARE\Node.js')) {
  if (Test-Path $key) {
    $props = Get-ItemProperty $key -ErrorAction SilentlyContinue
    if ($props -and $props.InstallPath) {
      [PSCustomObject]@{ Version = $props.Version; Path = [string]$props.InstallPath }
    }
  }
})

$pythonPaths = @(foreach ($root in @('HKLM:\SOFTWARE\Python\PythonCore', 'HKLM:\SOFTWARE\WOW6432Node\Python\PythonCore', 'HKCU:\SOFTWARE\Python\PythonCore')) {
  if (Test-Path $root) {
    foreach ($child in Get-ChildItem $root -ErrorAction SilentlyContinue) {
      $ipKey = Join-Path $child.PSPath 'InstallPath'
      if (Test-Path $ipKey) {
        $props = Get-ItemProperty $ipKey -ErrorAction SilentlyContinue
        if ($props) {
          $dir = $props.'(default)'
          $exe = $props.ExecutablePath
          if ($dir -or $exe) {
            [PSCustomObject]@{
              Tag = [string]$child.PSChildName
              InstallPath = if ($dir) { [string]$dir } else { $null }
              ExecutablePath = if ($exe) { [string]$exe } else { $null }
            }
          }
        }
      }
    }
  }
})

$gitPaths = @(foreach ($key in @('HKLM:\SOFTWARE\GitForWindows', 'HKLM:\SOFTWARE\WOW6432Node\GitForWindows', 'HKCU:\SOFTWARE\GitForWindows')) {
  if (Test-Path $key) {
    $props = Get-ItemProperty $key -ErrorAction SilentlyContinue
    if ($props -and $props.InstallPath) {
      [PSCustomObject]@{ Version = $props.CurrentVersion; Path = [string]$props.InstallPath }
    }
  }
})

$temurinPaths = @(foreach ($root in @('HKLM:\SOFTWARE\Eclipse Adoptium\JDK', 'HKCU:\SOFTWARE\Eclipse Adoptium\JDK')) {
  if (Test-Path $root) {
    foreach ($child in Get-ChildItem $root -ErrorAction SilentlyContinue) {
      $msiKey = Join-Path $child.PSPath 'hotspot\MSI'
      if (Test-Path $msiKey) {
        $props = Get-ItemProperty $msiKey -ErrorAction SilentlyContinue
        if ($props -and $props.Path) {
          [PSCustomObject]@{ Version = [string]$child.PSChildName; Path = [string]$props.Path }
        }
      }
    }
  }
})

$goPaths = @(foreach ($key in @('HKLM:\SOFTWARE\GoProgrammingLanguage', 'HKCU:\SOFTWARE\GoProgrammingLanguage')) {
  if (Test-Path $key) {
    $props = Get-ItemProperty $key -ErrorAction SilentlyContinue
    if ($props -and $props.installLocation) { [string]$props.installLocation }
  }
})

$cmakePaths = @(foreach ($key in @('HKLM:\SOFTWARE\Kitware\CMake', 'HKLM:\SOFTWARE\WOW6432Node\Kitware\CMake')) {
  if (Test-Path $key) {
    $props = Get-ItemProperty $key -ErrorAction SilentlyContinue
    if ($props -and $props.'(default)') { [string]$props.'(default)' }
  }
})

$dotnetPaths = @(foreach ($root in @('HKLM:\SOFTWARE\dotnet\Setup\InstalledVersions', 'HKLM:\SOFTWARE\WOW6432Node\dotnet\Setup\InstalledVersions')) {
  if (Test-Path $root) {
    foreach ($arch in Get-ChildItem $root -ErrorAction SilentlyContinue) {
      $props = Get-ItemProperty $arch.PSPath -ErrorAction SilentlyContinue
      if ($props -and $props.InstallLocation) { [string]$props.InstallLocation }
    }
  }
})

$result = [PSCustomObject]@{
  Applications = $apps
  NodePaths = $nodePaths
  PythonPaths = $pythonPaths
  GitPaths = $gitPaths
  TemurinPaths = $temurinPaths
  GoPaths = $goPaths
  CmakePaths = $cmakePaths
  DotnetPaths = $dotnetPaths
}
ConvertTo-Json -InputObject $result -Depth 4 -Compress
"#;

pub(super) async fn collect_inventory() -> Result<RegistryInventory> {
    let system = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::internal("无法确定 Windows 系统目录。"))?;
    let powershell = PathBuf::from(system)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let text = process::run(
        &powershell,
        &process::args(&[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            INVENTORY_SCRIPT,
        ]),
        false,
    )
    .await?
    .checked_stdout()?;
    let inventory: RegistryInventory = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|_| Error::internal("无法解析 Windows 已安装应用记录，未开放环境修改。"))?;
    if inventory
        .applications
        .iter()
        .any(|app| app.tool().is_some() && app.package().is_none())
    {
        return Err(Error::conflict(
            "已安装工具的 Windows 注册信息不完整，请先在系统设置中处理，避免重复安装。",
        ));
    }
    Ok(inventory)
}

impl RegistryInventory {
    pub(super) fn owned_files(&self, tool: &str) -> Vec<PathBuf> {
        self.applications
            .iter()
            .flat_map(|app| app.owned_files(tool, self))
            .collect()
    }

    fn supplementary_locations(
        &self,
        app: &InstalledApplication,
        installed_tool: &str,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut directories = Vec::new();
        let mut direct_files = Vec::new();
        match installed_tool {
            "node" => {
                let app_version = app.display_version.as_deref().map(normalize_version);
                for record in &self.node_paths {
                    let matches = match (
                        record.version.as_deref().map(normalize_version),
                        app_version,
                    ) {
                        (Some(left), Some(right)) => left == right,
                        (None, _) => self.node_paths.len() == 1,
                        _ => false,
                    };
                    if matches {
                        push_clean_dir(&mut directories, &record.path);
                    }
                }
            }
            "python" => {
                if let Some(minor) = app.python_minor() {
                    for record in &self.python_paths {
                        if python_tag_minor(&record.tag) == Some(minor.as_str()) {
                            if let Some(dir) = &record.install_path {
                                push_clean_dir(&mut directories, dir);
                            }
                            if let Some(exe) = &record.executable_path {
                                push_clean_dir(&mut direct_files, exe);
                            }
                        }
                    }
                }
            }
            "git" => {
                let app_version = app.display_version.as_deref().map(normalize_version);
                for record in &self.git_paths {
                    let matches = match (
                        record.version.as_deref().map(normalize_version),
                        app_version,
                    ) {
                        (Some(left), Some(right)) => left == right,
                        (None, _) => self.git_paths.len() == 1,
                        _ => false,
                    };
                    if matches {
                        push_clean_dir(&mut directories, &record.path);
                    }
                }
            }
            "java" => {
                if let Some(major) = app.major_version(4) {
                    for record in &self.temurin_paths {
                        let rec_major = record
                            .version
                            .as_deref()
                            .and_then(|value| value.split('.').next());
                        if rec_major == Some(major.as_str()) {
                            push_clean_dir(&mut directories, &record.path);
                        }
                    }
                }
            }
            "go" => {
                for dir in &self.go_paths {
                    push_clean_dir(&mut directories, dir);
                }
            }
            "cmake" => {
                for dir in &self.cmake_paths {
                    push_clean_dir(&mut directories, dir);
                }
            }
            "dotnet" => {
                for dir in &self.dotnet_paths {
                    push_clean_dir(&mut directories, dir);
                }
            }
            _ => {}
        }
        (directories, direct_files)
    }
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

fn python_tag_minor(tag: &str) -> Option<&str> {
    let base = tag.split('-').next()?;
    let mut parts = base.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    if parts.next().is_none()
        && !major.is_empty()
        && !minor.is_empty()
        && major.chars().all(|ch| ch.is_ascii_digit())
        && minor.chars().all(|ch| ch.is_ascii_digit())
    {
        Some(base)
    } else {
        None
    }
}

fn push_clean_dir(target: &mut Vec<PathBuf>, raw: &str) {
    let trimmed = raw.trim().trim_matches('"');
    if !trimmed.is_empty() {
        target.push(PathBuf::from(trimmed));
    }
}

fn name_matches_exact_or_versioned(name: &str, base: &str) -> bool {
    name == base
        || name
            .strip_prefix(base)
            .and_then(|rest| rest.strip_prefix(' '))
            .is_some_and(|rest| {
                rest.trim_start_matches('v')
                    .starts_with(|ch: char| ch.is_ascii_digit())
            })
}

fn extract_major_minor(text: &str) -> Option<String> {
    for token in text.split(|ch: char| !ch.is_ascii_digit() && ch != '.') {
        let parts: Vec<_> = token
            .split('.')
            .filter(|part| !part.is_empty())
            .take(2)
            .collect();
        if parts.len() == 2 {
            return Some(parts.join("."));
        }
    }
    None
}

impl InstalledApplication {
    fn tool(&self) -> Option<&'static str> {
        let name = self.display_name.trim().to_ascii_lowercase();
        if name == "git lfs" || name_matches_exact_or_versioned(&name, "git lfs") {
            Some("git-lfs")
        } else if name == "git"
            || name.starts_with("git version ")
            || name_matches_exact_or_versioned(&name, "git")
        {
            Some("git")
        } else if name.starts_with("github cli") {
            Some("gh")
        } else if name == "glab"
            || name.starts_with("gitlab cli")
            || name_matches_exact_or_versioned(&name, "glab")
        {
            Some("glab")
        } else if name_matches_exact_or_versioned(&name, "lazygit") {
            Some("lazygit")
        } else if name == "git-delta"
            || name_matches_exact_or_versioned(&name, "git-delta")
            || name_matches_exact_or_versioned(&name, "delta")
        {
            Some("delta")
        } else if name == "pnpm" || name.starts_with("pnpm ") {
            Some("pnpm")
        } else if name_matches_exact_or_versioned(&name, "yarn") {
            Some("yarn")
        } else if name_matches_exact_or_versioned(&name, "uv") {
            Some("uv")
        } else if name == "composer"
            || name.starts_with("composer -")
            || name_matches_exact_or_versioned(&name, "composer")
        {
            Some("composer")
        } else if name == "node.js" || name.starts_with("node.js ") {
            Some("node")
        } else if name.starts_with("python ") && !name.contains("launcher") {
            Some("python")
        } else if name.starts_with("rust ") || name.starts_with("rustup") {
            Some("rust")
        } else if name == "go" || name.starts_with("go programming language") {
            Some("go")
        } else if name.contains("temurin") {
            Some("java")
        } else if name.starts_with(".net sdk") || name.starts_with("microsoft .net sdk") {
            Some("dotnet")
        } else if name_matches_exact_or_versioned(&name, "ruby")
            && self.major_minor_version().is_some()
        {
            Some("ruby")
        } else if name_matches_exact_or_versioned(&name, "php")
            && self.major_minor_version().is_some()
        {
            Some("php")
        } else if name_matches_exact_or_versioned(&name, "zig") {
            Some("zig")
        } else if name_matches_exact_or_versioned(&name, "bun") {
            Some("bun")
        } else if name_matches_exact_or_versioned(&name, "deno") {
            Some("deno")
        } else if name == "cmake" || name.starts_with("cmake ") {
            Some("cmake")
        } else if name == "llvm" || name_matches_exact_or_versioned(&name, "llvm") {
            Some("clang")
        } else if name == "gnu make"
            || name.starts_with("gnu make ")
            || name.starts_with("gnuwin32: make")
            || name_matches_exact_or_versioned(&name, "make")
        {
            Some("make")
        } else if name_matches_exact_or_versioned(&name, "ninja") {
            Some("ninja")
        } else if name == "meson"
            || name.starts_with("meson build")
            || name_matches_exact_or_versioned(&name, "meson")
        {
            Some("meson")
        } else if name_matches_exact_or_versioned(&name, "just") {
            Some("just")
        } else if name == "protobuf"
            || name.starts_with("protocol buffers")
            || name_matches_exact_or_versioned(&name, "protobuf")
        {
            Some("protoc")
        } else if name == "docker desktop"
            || name == "docker cli"
            || name_matches_exact_or_versioned(&name, "docker cli")
        {
            Some("docker")
        } else if !name.contains("desktop") && name_matches_exact_or_versioned(&name, "podman") {
            Some("podman")
        } else if name_matches_exact_or_versioned(&name, "kubectl")
            || name_matches_exact_or_versioned(&name, "kubernetes cli")
        {
            Some("kubectl")
        } else if name_matches_exact_or_versioned(&name, "helm") {
            Some("helm")
        } else if name_matches_exact_or_versioned(&name, "terraform") {
            Some("terraform")
        } else if name == "sqlite"
            || name == "sqlite3"
            || name_matches_exact_or_versioned(&name, "sqlite")
            || name_matches_exact_or_versioned(&name, "sqlite3")
        {
            Some("sqlite")
        } else if name_matches_exact_or_versioned(&name, "neovim") {
            Some("neovim")
        } else if name_matches_exact_or_versioned(&name, "vim") {
            Some("vim")
        } else if name == "ripgrep" || name.starts_with("ripgrep ") {
            Some("ripgrep")
        } else if name_matches_exact_or_versioned(&name, "fd") {
            Some("fd")
        } else if name_matches_exact_or_versioned(&name, "bat") {
            Some("bat")
        } else if name_matches_exact_or_versioned(&name, "jq") {
            Some("jq")
        } else if name_matches_exact_or_versioned(&name, "fzf") {
            Some("fzf")
        } else if name_matches_exact_or_versioned(&name, "shellcheck") {
            Some("shellcheck")
        } else if name_matches_exact_or_versioned(&name, "shfmt") {
            Some("shfmt")
        } else if name_matches_exact_or_versioned(&name, "starship") {
            Some("starship")
        } else if name_matches_exact_or_versioned(&name, "zoxide") {
            Some("zoxide")
        } else if name_matches_exact_or_versioned(&name, "eza") {
            Some("eza")
        } else {
            None
        }
    }

    fn major_minor_version(&self) -> Option<String> {
        self.display_version
            .as_deref()
            .and_then(extract_major_minor)
            .or_else(|| extract_major_minor(&self.display_name))
    }

    fn python_minor(&self) -> Option<String> {
        let version = self
            .display_version
            .as_deref()
            .or_else(|| self.display_name.split_whitespace().nth(1))?;
        let parts: Vec<_> = version.split('.').take(2).collect();
        if parts.len() != 2
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
        {
            return None;
        }
        Some(parts.join("."))
    }

    fn major_version(&self, name_token_index: usize) -> Option<String> {
        let version = self
            .display_version
            .as_deref()
            .or_else(|| self.display_name.split_whitespace().nth(name_token_index))?;
        let major = version.split('.').next()?;
        if major.is_empty() || !major.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        Some(major.to_owned())
    }

    pub(super) fn package(&self) -> Option<Package> {
        let lower = self.display_name.to_ascii_lowercase();
        let name = match self.tool()? {
            "git" => "Git.Git".into(),
            "git-lfs" => "GitHub.GitLFS".into(),
            "gh" => "GitHub.cli".into(),
            "glab" => "GLab.GLab".into(),
            "lazygit" => "JesseDuffield.lazygit".into(),
            "delta" => "dandavison.delta".into(),
            "node" => "OpenJS.NodeJS".into(),
            "pnpm" => "pnpm.pnpm".into(),
            "yarn" => "YarnPkg.Yarn".into(),
            "uv" => "astral-sh.uv".into(),
            "composer" => "Composer.Composer".into(),
            "python" => format!("Python.Python.{}", self.python_minor()?),
            "rust" if lower.contains("rustup") => "Rustlang.Rustup".into(),
            "rust" if lower.contains("gnu") => "Rustlang.Rust.GNU".into(),
            "rust" => "Rustlang.Rust.MSVC".into(),
            "go" => "GoLang.Go".into(),
            "java" => format!("EclipseAdoptium.Temurin.{}.JDK", self.major_version(4)?),
            "dotnet" => format!("Microsoft.DotNet.SDK.{}", self.major_version(2)?),
            "ruby" if lower.contains("msys2") || lower.contains("devkit") => {
                format!(
                    "RubyInstallerTeam.RubyWithDevKit.{}",
                    self.major_minor_version()?
                )
            }
            "ruby" => format!("RubyInstallerTeam.Ruby.{}", self.major_minor_version()?),
            "php" => format!("PHP.PHP.{}", self.major_minor_version()?),
            "zig" => "zig.zig".into(),
            "bun" => "Oven-sh.Bun".into(),
            "deno" => "DenoLand.Deno".into(),
            "cmake" => "Kitware.CMake".into(),
            "clang" => "LLVM.LLVM".into(),
            "make" if lower.contains("gnuwin32") => "GnuWin32.Make".into(),
            "make" => "ezwinports.make".into(),
            "ninja" => "Ninja-build.Ninja".into(),
            "meson" => "mesonbuild.meson".into(),
            "just" => "Casey.Just".into(),
            "protoc" => "Google.Protobuf".into(),
            "docker" if lower.contains("desktop") => "Docker.DockerDesktop".into(),
            "docker" => "Docker.DockerCLI".into(),
            "podman" => "RedHat.Podman".into(),
            "kubectl" => "Kubernetes.kubectl".into(),
            "helm" => "Helm.Helm".into(),
            "terraform" => "Hashicorp.Terraform".into(),
            "sqlite" => "SQLite.SQLite".into(),
            "neovim" => "Neovim.Neovim".into(),
            "vim" => "vim.vim".into(),
            "ripgrep" if lower.contains("gnu") => "BurntSushi.ripgrep.GNU".into(),
            "ripgrep" => "BurntSushi.ripgrep.MSVC".into(),
            "fd" => "sharkdp.fd".into(),
            "bat" => "sharkdp.bat".into(),
            "jq" => "jqlang.jq".into(),
            "fzf" => "junegunn.fzf".into(),
            "shellcheck" => "koalaman.shellcheck".into(),
            "shfmt" => "mvdan.shfmt".into(),
            "starship" => "Starship.Starship".into(),
            "zoxide" => "ajeetdsouza.zoxide".into(),
            "eza" => "eza-community.eza".into(),
            _ => return None,
        };
        Some(Package {
            name,
            version: self
                .display_version
                .clone()
                .unwrap_or_else(|| "未知版本".into()),
        })
    }

    fn owned_files(&self, tool: &str, inventory: &RegistryInventory) -> Vec<PathBuf> {
        let Some(installed) = self.tool() else {
            return Vec::new();
        };
        // Git for Windows ships Git LFS inside the Git installation.
        let bundled = tool == "git-lfs" && installed == "git";
        if installed != tool && !bundled {
            return Vec::new();
        }
        let mut directories = Vec::new();
        let mut install_root = None;
        if let Some(location) = &self.install_location {
            let trimmed = location.trim().trim_matches('"');
            if !trimmed.is_empty() {
                let path = PathBuf::from(trimmed);
                install_root = Some(path.clone());
                directories.push(path);
            }
        }
        if let Some(icon) = &self.display_icon {
            let icon = icon.trim().trim_matches('"');
            let icon = icon
                .rsplit_once(',')
                .filter(|(_, index)| index.parse::<i32>().is_ok())
                .map_or(icon, |(path, _)| path)
                .trim_matches('"');
            if let Some(parent) = Path::new(icon).parent() {
                directories.push(parent.to_path_buf());
            }
        }
        let (extra_dirs, direct_files) = inventory.supplementary_locations(self, installed);
        directories.extend(extra_dirs);

        let names: &[&str] = match if bundled { "git-lfs" } else { tool } {
            "git" => &[
                "cmd/git.exe",
                "bin/git.exe",
                "git.exe",
                "mingw64/bin/git.exe",
            ],
            "git-lfs" => &[
                "cmd/git-lfs.exe",
                "bin/git-lfs.exe",
                "git-lfs.exe",
                "mingw64/bin/git-lfs.exe",
            ],
            "gh" => &["gh.exe", "bin/gh.exe"],
            "glab" => &["glab.exe", "bin/glab.exe"],
            "lazygit" => &["lazygit.exe", "bin/lazygit.exe"],
            "delta" => &["delta.exe", "bin/delta.exe"],
            "node" => &["node.exe"],
            "pnpm" => &["pnpm.exe", "pnpm.cmd"],
            "yarn" => &["bin/yarn.cmd", "yarn.cmd", "yarn.exe"],
            "uv" => &["uv.exe", "bin/uv.exe"],
            "composer" => &["composer.bat", "composer.cmd", "bin/composer.bat"],
            "python" => &["python.exe", "python3.exe"],
            "rust" => &["bin/cargo.exe", "bin/rustc.exe", "cargo.exe", "rustc.exe"],
            "go" => &["bin/go.exe", "go.exe"],
            "java" => &["bin/java.exe", "bin/javac.exe", "java.exe", "javac.exe"],
            "dotnet" => &["dotnet.exe"],
            "ruby" => &["bin/ruby.exe", "ruby.exe"],
            "php" => &["php.exe", "bin/php.exe"],
            "zig" => &["zig.exe", "bin/zig.exe"],
            "bun" => &["bun.exe", "bin/bun.exe"],
            "deno" => &["deno.exe", "bin/deno.exe"],
            "cmake" => &["bin/cmake.exe", "cmake.exe"],
            "clang" => &["bin/clang.exe", "clang.exe"],
            "make" => &["bin/make.exe", "make.exe"],
            "ninja" => &["ninja.exe", "bin/ninja.exe"],
            "meson" => &["meson.exe", "bin/meson.exe"],
            "just" => &["just.exe", "bin/just.exe"],
            "protoc" => &["bin/protoc.exe", "protoc.exe"],
            "docker" => &["resources/bin/docker.exe", "bin/docker.exe", "docker.exe"],
            "podman" => &["podman.exe", "bin/podman.exe"],
            "kubectl" => &["kubectl.exe", "bin/kubectl.exe"],
            "helm" => &["helm.exe", "bin/helm.exe"],
            "terraform" => &["terraform.exe", "bin/terraform.exe"],
            "sqlite" => &["sqlite3.exe", "bin/sqlite3.exe"],
            "neovim" => &["bin/nvim.exe", "nvim.exe"],
            "vim" => &["vim.exe", "bin/vim.exe"],
            "ripgrep" => &["rg.exe", "bin/rg.exe"],
            "fd" => &["fd.exe", "bin/fd.exe"],
            "bat" => &["bat.exe", "bin/bat.exe"],
            "jq" => &["jq.exe", "bin/jq.exe"],
            "fzf" => &["fzf.exe", "bin/fzf.exe"],
            "shellcheck" => &["shellcheck.exe", "bin/shellcheck.exe"],
            "shfmt" => &["shfmt.exe", "bin/shfmt.exe"],
            "starship" => &["starship.exe", "bin/starship.exe"],
            "zoxide" => &["zoxide.exe", "bin/zoxide.exe"],
            "eza" => &["eza.exe", "bin/eza.exe"],
            _ => &[],
        };
        // Portable WinGet zip packages extract into a versioned child directory
        // under InstallLocation (e.g. `ripgrep-14.1.1-.../rg.exe`).
        if let Some(root) = install_root
            && root.is_absolute()
            && root.components().count() >= 4
            && let Ok(entries) = std::fs::read_dir(&root)
        {
            for entry in entries.flatten().take(16) {
                let child = entry.path();
                if child.is_dir() {
                    directories.push(child);
                }
            }
        }
        let mut files: Vec<PathBuf> = directories
            .into_iter()
            .filter(|path| path.is_absolute())
            .flat_map(|directory| {
                names.iter().map(move |name| {
                    name.split('/')
                        .fold(directory.clone(), |acc, part| acc.join(part))
                })
            })
            .filter(|path| paths::is_executable(path))
            .collect();
        if !bundled {
            files.extend(
                direct_files
                    .into_iter()
                    .filter(|path| path.is_absolute() && paths::is_executable(path)),
            );
        }
        files.sort();
        files.dedup();
        files
    }
}
