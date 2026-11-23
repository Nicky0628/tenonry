use crate::error::{Error, Result};
use crate::infra::process;
use std::path::Path;

pub(super) fn supports_tool(tool_id: &str) -> bool {
    matches!(
        tool_id,
        // Languages & runtimes
        "python"
            | "node"
            | "rust"
            | "go"
            | "java"
            | "dotnet"
            | "ruby"
            | "php"
            | "zig"
            | "bun"
            | "deno"
            // Package managers
            | "pnpm"
            | "yarn"
            | "uv"
            | "composer"
            // Version control
            | "git"
            | "git-lfs"
            | "gh"
            | "glab"
            | "lazygit"
            | "delta"
            // Build & compilers
            | "cmake"
            | "clang"
            | "make"
            | "ninja"
            | "meson"
            | "just"
            | "protoc"
            // Containers & cloud native
            | "docker"
            | "podman"
            | "kubectl"
            | "helm"
            | "terraform"
            // Database clients
            | "sqlite"
            // Editors & terminal CLI
            | "neovim"
            | "vim"
            | "ripgrep"
            | "fd"
            | "bat"
            | "jq"
            | "fzf"
            | "shellcheck"
            | "shfmt"
            | "starship"
            | "zoxide"
            | "eza"
    )
}

pub(super) fn matches_tool(tool_id: &str, package: &str) -> bool {
    match tool_id {
        // Languages & runtimes
        "python" => package.starts_with("Python.Python."),
        "node" => matches!(package, "OpenJS.NodeJS" | "OpenJS.NodeJS.LTS"),
        "rust" => package.starts_with("Rustlang.Rust"),
        "go" => package == "GoLang.Go",
        "java" => package.starts_with("EclipseAdoptium.Temurin."),
        "dotnet" => package.starts_with("Microsoft.DotNet.SDK."),
        "ruby" => {
            package.starts_with("RubyInstallerTeam.Ruby.")
                || package.starts_with("RubyInstallerTeam.RubyWithDevKit.")
        }
        "php" => package.starts_with("PHP.PHP."),
        "zig" => package == "zig.zig",
        "bun" => package == "Oven-sh.Bun",
        "deno" => package == "DenoLand.Deno",

        // Package managers
        "pnpm" => package == "pnpm.pnpm",
        "yarn" => package == "YarnPkg.Yarn",
        "uv" => package == "astral-sh.uv",
        "composer" => package == "Composer.Composer",

        // Version control
        "git" => package == "Git.Git",
        "git-lfs" => package == "GitHub.GitLFS",
        "gh" => package == "GitHub.cli",
        "glab" => package == "GLab.GLab",
        "lazygit" => package == "JesseDuffield.lazygit",
        "delta" => package == "dandavison.delta",

        // Build & compilers
        "cmake" => package == "Kitware.CMake",
        "clang" => package == "LLVM.LLVM",
        "make" => matches!(package, "ezwinports.make" | "GnuWin32.Make"),
        "ninja" => package == "Ninja-build.Ninja",
        "meson" => package == "mesonbuild.meson",
        "just" => package == "Casey.Just",
        "protoc" => package == "Google.Protobuf",

        // Containers & cloud native
        "docker" => matches!(package, "Docker.DockerCLI" | "Docker.DockerDesktop"),
        "podman" => package == "RedHat.Podman",
        "kubectl" => package == "Kubernetes.kubectl",
        "helm" => package == "Helm.Helm",
        "terraform" => package == "Hashicorp.Terraform",

        // Database clients
        "sqlite" => package == "SQLite.SQLite",

        // Editors & terminal CLI
        "neovim" => package == "Neovim.Neovim",
        "vim" => package == "vim.vim",
        "ripgrep" => matches!(
            package,
            "BurntSushi.ripgrep.MSVC" | "BurntSushi.ripgrep.GNU"
        ),
        "fd" => package == "sharkdp.fd",
        "bat" => package == "sharkdp.bat",
        "jq" => package == "jqlang.jq",
        "fzf" => package == "junegunn.fzf",
        "shellcheck" => package == "koalaman.shellcheck",
        "shfmt" => package == "mvdan.shfmt",
        "starship" => package == "Starship.Starship",
        "zoxide" => package == "ajeetdsouza.zoxide",
        "eza" => package == "eza-community.eza",
        _ => false,
    }
}

pub(super) async fn default_packages(winget: &Path, tool_id: &str) -> Result<Vec<String>> {
    let id = match tool_id {
        // Languages & runtimes
        "node" => "OpenJS.NodeJS.LTS",
        "rust" => "Rustlang.Rust.MSVC",
        "go" => "GoLang.Go",
        "zig" => "zig.zig",
        "bun" => "Oven-sh.Bun",
        "deno" => "DenoLand.Deno",
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

        // Package managers
        "pnpm" => "pnpm.pnpm",
        "yarn" => "YarnPkg.Yarn",
        "uv" => "astral-sh.uv",
        "composer" => "Composer.Composer",

        // Version control
        "git" => "Git.Git",
        "git-lfs" => "GitHub.GitLFS",
        "gh" => "GitHub.cli",
        "glab" => "GLab.GLab",
        "lazygit" => "JesseDuffield.lazygit",
        "delta" => "dandavison.delta",

        // Build & compilers
        "cmake" => "Kitware.CMake",
        "clang" => "LLVM.LLVM",
        "make" => "ezwinports.make",
        "ninja" => "Ninja-build.Ninja",
        "meson" => "mesonbuild.meson",
        "just" => "Casey.Just",
        "protoc" => "Google.Protobuf",

        // Containers & cloud native
        "docker" => "Docker.DockerCLI",
        "podman" => "RedHat.Podman",
        "kubectl" => "Kubernetes.kubectl",
        "helm" => "Helm.Helm",
        "terraform" => "Hashicorp.Terraform",

        // Database clients
        "sqlite" => "SQLite.SQLite",

        // Editors & terminal CLI
        "neovim" => "Neovim.Neovim",
        "vim" => "vim.vim",
        "ripgrep" => "BurntSushi.ripgrep.MSVC",
        "fd" => "sharkdp.fd",
        "bat" => "sharkdp.bat",
        "jq" => "jqlang.jq",
        "fzf" => "junegunn.fzf",
        "shellcheck" => "koalaman.shellcheck",
        "shfmt" => "mvdan.shfmt",
        "starship" => "Starship.Starship",
        "zoxide" => "ajeetdsouza.zoxide",
        "eza" => "eza-community.eza",
        _ => return Err(Error::unsupported("WinGet 暂不支持此工具。")),
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
