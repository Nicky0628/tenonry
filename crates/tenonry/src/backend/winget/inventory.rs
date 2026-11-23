use super::mapping;
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
        if name == "node.js" || name.starts_with("node.js ") {
            return Some("node");
        }
        if name.starts_with("python ") && !name.contains("launcher") {
            return Some("python");
        }
        if name.starts_with("rust ") || name.starts_with("rustup") {
            return Some("rust");
        }
        if name.contains("temurin") {
            return Some("java");
        }
        if name.starts_with(".net sdk") || name.starts_with("microsoft .net sdk") {
            return Some("dotnet");
        }
        if name_matches_exact_or_versioned(&name, "ruby") && self.major_minor_version().is_some() {
            return Some("ruby");
        }
        if name_matches_exact_or_versioned(&name, "php") && self.major_minor_version().is_some() {
            return Some("php");
        }
        if name == "docker desktop" {
            return Some("docker");
        }
        if name.contains("desktop") && name.starts_with("podman") {
            return None;
        }
        mapping::FIXED_APPS.iter().find_map(|app| {
            let matched = app
                .exact_names
                .iter()
                .any(|base| name_matches_exact_or_versioned(&name, base))
                || app
                    .prefix_names
                    .iter()
                    .any(|prefix| name.starts_with(prefix));
            matched.then_some(app.tool_id)
        })
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
        let tool = self.tool()?;
        let name = match tool {
            "node" => "OpenJS.NodeJS".into(),
            "python" => format!("Python.Python.{}", self.python_minor()?),
            "rust" if lower.contains("rustup") => "Rustlang.Rustup".into(),
            "rust" if lower.contains("gnu") => "Rustlang.Rust.GNU".into(),
            "rust" => "Rustlang.Rust.MSVC".into(),
            "java" => format!("EclipseAdoptium.Temurin.{}.JDK", self.major_version(4)?),
            "dotnet" => format!("Microsoft.DotNet.SDK.{}", self.major_version(2)?),
            "ruby" if lower.contains("msys2") || lower.contains("devkit") => format!(
                "RubyInstallerTeam.RubyWithDevKit.{}",
                self.major_minor_version()?
            ),
            "ruby" => format!("RubyInstallerTeam.Ruby.{}", self.major_minor_version()?),
            "php" => format!("PHP.PHP.{}", self.major_minor_version()?),
            "make" if lower.contains("gnuwin32") => "GnuWin32.Make".into(),
            "docker" if lower.contains("desktop") => "Docker.DockerDesktop".into(),
            "ripgrep" if lower.contains("gnu") => "BurntSushi.ripgrep.GNU".into(),
            _ => mapping::find_fixed(tool)?.package_id.into(),
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

        let target = if bundled { "git-lfs" } else { tool };
        let names: &[&str] = match target {
            "node" => &["node.exe"],
            "python" => &["python.exe", "python3.exe"],
            "rust" => &["bin/cargo.exe", "bin/rustc.exe", "cargo.exe", "rustc.exe"],
            "java" => &["bin/java.exe", "bin/javac.exe", "java.exe", "javac.exe"],
            "dotnet" => &["dotnet.exe"],
            "ruby" => &["bin/ruby.exe", "ruby.exe"],
            "php" => &["php.exe", "bin/php.exe"],
            _ => mapping::find_fixed(target).map_or(&[], |app| app.exes),
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
