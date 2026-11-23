use crate::error::{Error, Result};
use crate::infra::process;
use std::path::Path;

/// Declarative mapping for WinGet tools with fixed package IDs, shared across
/// plan generation and Windows Uninstall registry ownership matching.
pub(super) struct FixedApp {
    pub(super) tool_id: &'static str,
    pub(super) package_id: &'static str,
    pub(super) extra_ids: &'static [&'static str],
    pub(super) exact_names: &'static [&'static str],
    pub(super) prefix_names: &'static [&'static str],
    pub(super) exes: &'static [&'static str],
}

impl FixedApp {
    const fn new(
        tool_id: &'static str,
        package_id: &'static str,
        exact_names: &'static [&'static str],
        exes: &'static [&'static str],
    ) -> Self {
        Self {
            tool_id,
            package_id,
            extra_ids: &[],
            exact_names,
            prefix_names: &[],
            exes,
        }
    }

    const fn extra(mut self, ids: &'static [&'static str]) -> Self {
        self.extra_ids = ids;
        self
    }

    const fn prefixes(mut self, names: &'static [&'static str]) -> Self {
        self.prefix_names = names;
        self
    }
}

pub(super) const FIXED_APPS: &[FixedApp] = &[
    FixedApp::new("go", "GoLang.Go", &["go"], &["bin/go.exe", "go.exe"])
        .prefixes(&["go programming language"]),
    FixedApp::new("zig", "zig.zig", &["zig"], &["zig.exe", "bin/zig.exe"]),
    FixedApp::new("bun", "Oven-sh.Bun", &["bun"], &["bun.exe", "bin/bun.exe"]),
    FixedApp::new(
        "deno",
        "DenoLand.Deno",
        &["deno"],
        &["deno.exe", "bin/deno.exe"],
    ),
    FixedApp::new("pnpm", "pnpm.pnpm", &["pnpm"], &["pnpm.exe", "pnpm.cmd"]).prefixes(&["pnpm "]),
    FixedApp::new(
        "yarn",
        "YarnPkg.Yarn",
        &["yarn"],
        &["bin/yarn.cmd", "yarn.cmd", "yarn.exe"],
    ),
    FixedApp::new("uv", "astral-sh.uv", &["uv"], &["uv.exe", "bin/uv.exe"]),
    FixedApp::new(
        "composer",
        "Composer.Composer",
        &["composer"],
        &["composer.bat", "composer.cmd", "bin/composer.bat"],
    )
    .prefixes(&["composer -"]),
    FixedApp::new(
        "git-lfs",
        "GitHub.GitLFS",
        &["git lfs"],
        &[
            "cmd/git-lfs.exe",
            "bin/git-lfs.exe",
            "git-lfs.exe",
            "mingw64/bin/git-lfs.exe",
        ],
    ),
    FixedApp::new(
        "git",
        "Git.Git",
        &["git"],
        &[
            "cmd/git.exe",
            "bin/git.exe",
            "git.exe",
            "mingw64/bin/git.exe",
        ],
    )
    .prefixes(&["git version "]),
    FixedApp::new("gh", "GitHub.cli", &[], &["gh.exe", "bin/gh.exe"]).prefixes(&["github cli"]),
    FixedApp::new(
        "glab",
        "GLab.GLab",
        &["glab"],
        &["glab.exe", "bin/glab.exe"],
    )
    .prefixes(&["gitlab cli"]),
    FixedApp::new(
        "lazygit",
        "JesseDuffield.lazygit",
        &["lazygit"],
        &["lazygit.exe", "bin/lazygit.exe"],
    ),
    FixedApp::new(
        "delta",
        "dandavison.delta",
        &["git-delta", "delta"],
        &["delta.exe", "bin/delta.exe"],
    ),
    FixedApp::new(
        "cmake",
        "Kitware.CMake",
        &["cmake"],
        &["bin/cmake.exe", "cmake.exe"],
    )
    .prefixes(&["cmake "]),
    FixedApp::new(
        "clang",
        "LLVM.LLVM",
        &["llvm"],
        &["bin/clang.exe", "clang.exe"],
    ),
    FixedApp::new(
        "make",
        "ezwinports.make",
        &["gnu make", "make"],
        &["bin/make.exe", "make.exe"],
    )
    .extra(&["GnuWin32.Make"])
    .prefixes(&["gnu make ", "gnuwin32: make"]),
    FixedApp::new(
        "ninja",
        "Ninja-build.Ninja",
        &["ninja"],
        &["ninja.exe", "bin/ninja.exe"],
    ),
    FixedApp::new(
        "meson",
        "mesonbuild.meson",
        &["meson"],
        &["meson.exe", "bin/meson.exe"],
    )
    .prefixes(&["meson build"]),
    FixedApp::new(
        "just",
        "Casey.Just",
        &["just"],
        &["just.exe", "bin/just.exe"],
    ),
    FixedApp::new(
        "protoc",
        "Google.Protobuf",
        &["protobuf"],
        &["bin/protoc.exe", "protoc.exe"],
    )
    .prefixes(&["protocol buffers"]),
    FixedApp::new(
        "docker",
        "Docker.DockerCLI",
        &["docker cli"],
        &["resources/bin/docker.exe", "bin/docker.exe", "docker.exe"],
    )
    .extra(&["Docker.DockerDesktop"]),
    FixedApp::new(
        "podman",
        "RedHat.Podman",
        &["podman"],
        &["podman.exe", "bin/podman.exe"],
    ),
    FixedApp::new(
        "kubectl",
        "Kubernetes.kubectl",
        &["kubectl", "kubernetes cli"],
        &["kubectl.exe", "bin/kubectl.exe"],
    ),
    FixedApp::new(
        "helm",
        "Helm.Helm",
        &["helm"],
        &["helm.exe", "bin/helm.exe"],
    ),
    FixedApp::new(
        "terraform",
        "Hashicorp.Terraform",
        &["terraform"],
        &["terraform.exe", "bin/terraform.exe"],
    ),
    FixedApp::new(
        "sqlite",
        "SQLite.SQLite",
        &["sqlite", "sqlite3"],
        &["sqlite3.exe", "bin/sqlite3.exe"],
    ),
    FixedApp::new(
        "neovim",
        "Neovim.Neovim",
        &["neovim"],
        &["bin/nvim.exe", "nvim.exe"],
    ),
    FixedApp::new("vim", "vim.vim", &["vim"], &["vim.exe", "bin/vim.exe"]),
    FixedApp::new(
        "ripgrep",
        "BurntSushi.ripgrep.MSVC",
        &["ripgrep"],
        &["rg.exe", "bin/rg.exe"],
    )
    .extra(&["BurntSushi.ripgrep.GNU"])
    .prefixes(&["ripgrep "]),
    FixedApp::new("fd", "sharkdp.fd", &["fd"], &["fd.exe", "bin/fd.exe"]),
    FixedApp::new("bat", "sharkdp.bat", &["bat"], &["bat.exe", "bin/bat.exe"]),
    FixedApp::new("jq", "jqlang.jq", &["jq"], &["jq.exe", "bin/jq.exe"]),
    FixedApp::new("fzf", "junegunn.fzf", &["fzf"], &["fzf.exe", "bin/fzf.exe"]),
    FixedApp::new(
        "shellcheck",
        "koalaman.shellcheck",
        &["shellcheck"],
        &["shellcheck.exe", "bin/shellcheck.exe"],
    ),
    FixedApp::new(
        "shfmt",
        "mvdan.shfmt",
        &["shfmt"],
        &["shfmt.exe", "bin/shfmt.exe"],
    ),
    FixedApp::new(
        "starship",
        "Starship.Starship",
        &["starship"],
        &["starship.exe", "bin/starship.exe"],
    ),
    FixedApp::new(
        "zoxide",
        "ajeetdsouza.zoxide",
        &["zoxide"],
        &["zoxide.exe", "bin/zoxide.exe"],
    ),
    FixedApp::new(
        "eza",
        "eza-community.eza",
        &["eza"],
        &["eza.exe", "bin/eza.exe"],
    ),
];

pub(super) fn find_fixed(tool_id: &str) -> Option<&'static FixedApp> {
    FIXED_APPS.iter().find(|app| app.tool_id == tool_id)
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "python" | "node" | "rust" | "java" | "dotnet" | "ruby" | "php"
    ) || find_fixed(tool_id).is_some()
}

pub(super) fn matches_tool(tool_id: &str, package: &str) -> bool {
    match tool_id {
        "python" => package.starts_with("Python.Python."),
        "node" => matches!(package, "OpenJS.NodeJS" | "OpenJS.NodeJS.LTS"),
        "rust" => package.starts_with("Rustlang.Rust"),
        "java" => package.starts_with("EclipseAdoptium.Temurin."),
        "dotnet" => package.starts_with("Microsoft.DotNet.SDK."),
        "ruby" => {
            package.starts_with("RubyInstallerTeam.Ruby.")
                || package.starts_with("RubyInstallerTeam.RubyWithDevKit.")
        }
        "php" => package.starts_with("PHP.PHP."),
        _ => find_fixed(tool_id)
            .is_some_and(|app| package == app.package_id || app.extra_ids.contains(&package)),
    }
}

pub(super) async fn default_packages(winget: &Path, tool_id: &str) -> Result<Vec<String>> {
    let id = match tool_id {
        "node" => "OpenJS.NodeJS.LTS",
        "rust" => "Rustlang.Rust.MSVC",
        "java" => {
            return Ok(vec![
                newest_id(winget, "EclipseAdoptium.Temurin.", ".JDK").await?,
            ]);
        }
        "dotnet" => return Ok(vec![newest_id(winget, "Microsoft.DotNet.SDK.", "").await?]),
        "python" => return Ok(vec![newest_id(winget, "Python.Python.3.", "").await?]),
        "ruby" => {
            return Ok(vec![
                newest_id(winget, "RubyInstallerTeam.Ruby.", "").await?,
            ]);
        }
        "php" => return Ok(vec![newest_id(winget, "PHP.PHP.", "").await?]),
        _ => find_fixed(tool_id)
            .map(|app| app.package_id)
            .ok_or_else(|| Error::unsupported("WinGet 暂不支持此工具。"))?,
    };
    Ok(vec![id.into()])
}

/// Finds the newest numeric WinGet package ID matching `<prefix><version><suffix>`,
/// where `<version>` is one or more dot-separated integers (e.g. `21`, `3.4`, `8.4`).
async fn newest_id(winget: &Path, prefix: &str, suffix: &str) -> Result<String> {
    let output = process::run(
        winget,
        &process::args(&[
            "search",
            "--id",
            prefix,
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]),
        false,
    )
    .await?
    .checked()?;
    output
        .split_whitespace()
        .filter_map(|word| {
            let rest = word.strip_prefix(prefix)?;
            let version = if suffix.is_empty() {
                rest
            } else {
                rest.strip_suffix(suffix)?
            };
            let parts: Vec<u32> = version
                .split('.')
                .map(|part| part.parse::<u32>().ok())
                .collect::<Option<Vec<_>>>()?;
            (!parts.is_empty()).then(|| (parts, word.to_owned()))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, id)| id)
        .ok_or_else(|| Error::not_found(format!("WinGet 没有返回可安装的 {prefix} 版本。")))
}

pub(super) async fn resolve_node_id(winget: &Path) -> Result<&'static str> {
    for id in ["OpenJS.NodeJS.LTS", "OpenJS.NodeJS"] {
        let output = process::run(
            winget,
            &process::args(&[
                "list",
                "--id",
                id,
                "--exact",
                "--source",
                "winget",
                "--accept-source-agreements",
                "--disable-interactivity",
            ]),
            false,
        )
        .await?;
        if output.status.success()
            && output
                .text
                .lines()
                .any(|line| line.split_whitespace().any(|word| word == id))
        {
            return Ok(id);
        }
        // APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND is an ordinary miss.
        if !output.status.success()
            && output.status.code().map(|code| code as u32) != Some(0x8A150014)
        {
            return Err(output.checked().unwrap_err());
        }
    }
    Err(Error::conflict(
        "已安装 Node.js，但 WinGet 无法确认它的包标识。请先通过 Windows 设置处理。",
    ))
}

pub(super) async fn verify_package(winget: &Path, id: &str) -> Result<()> {
    let output = process::run(
        winget,
        &process::args(&[
            "list",
            "--id",
            id,
            "--exact",
            "--source",
            "winget",
            "--accept-source-agreements",
            "--disable-interactivity",
        ]),
        false,
    )
    .await?
    .checked_stdout()?;
    if output
        .lines()
        .any(|line| line.split_whitespace().any(|word| word == id))
    {
        Ok(())
    } else {
        Err(Error::conflict(format!(
            "WinGet 无法精确识别 {id}，请通过 Windows 设置处理此安装。"
        )))
    }
}
