# Tenonry

[English](README.md) | 简体中文

[![CI](https://github.com/Nicky0628/tenonry/actions/workflows/check.yml/badge.svg)](https://github.com/Nicky0628/tenonry/actions/workflows/check.yml)
[![Crates.io](https://img.shields.io/crates/v/tenonry.svg)](https://crates.io/crates/tenonry)
[![Documentation](https://docs.rs/tenonry/badge.svg)](https://docs.rs/tenonry)
[![License](https://img.shields.io/crates/l/tenonry.svg)](#开源协议)

跨平台开发工具环境协调库与桌面工具（Cargo Workspace）。建立在现有系统包管理器与生态包管理器之上，提供工具探测、安装来源归属、版本查询、计划预检、多后端批量编排、任务日志与结果复核。

## 支持范围

| 后端 | 作用域 | 平台 | 安装、更新、卸载 | 指定版本 | 事务合并 |
| --- | --- | --- | --- | --- | --- |
| APT | System | Linux | 支持 | 支持 | 支持 |
| DNF | System | Linux | 支持 | — | 支持 |
| pacman | System | Linux | 支持 | — | 支持 |
| WinGet | System | Windows | 支持 | — | 支持 |
| npm | Ecosystem | Linux、Windows | 支持 | 支持 | — |

可用工具与版本由工具注册表、本机可用后端与软件源共同决定；操作严格遵循工具的实际安装来源，系统受保护组件和多来源冲突工具禁止自动修改。

> 平台说明：核心库在 Linux、Windows 与 macOS 下均通过编译检查；当前原生包管理器后端面向 Linux（APT、DNF、pacman、npm）与 Windows（WinGet、npm）。

## 工作区结构

```text
tenonry/
├── Cargo.toml                  # Workspace 根配置
├── crates/
│   └── tenonry/                # 核心 Rust 库（零 UI 依赖，发布于 crates.io）
│       └── src/
│           ├── lib.rs          # 对外门面 Tenonry 与公开导出
│           ├── error.rs        # 结构化错误 Error / ErrorKind
│           ├── model/          # 强类型领域模型与序列化 DTO
│           ├── infra/          # 进程、路径、探测、锁、提权、原子读写
│           ├── registry/       # 与包管理器无关的工具目录与通用规则
│           ├── backend/        # BackendDriver 契约与各后端纵向实现
│           └── engine/         # 探测、更新、版本、计划、执行、核验与任务日志
├── apps/
│   └── desktop/                # 独立 Tauri 2 桌面应用（原生 HTML/CSS/ES Modules）
│       ├── src/                # 单实例、来源校验、IPC 适配与任务关闭保护
│       └── ui/                 # 工具、后端与软件源、任务记录及操作弹窗界面
└── scripts/
    └── check-ui.mjs            # 界面 ES 模块语法与静态资源引用检查
```

## 使用核心库

需要 Rust **1.89+**（Edition 2024）稳定工具链。执行包管理操作时，本机需已安装对应的包管理器。

```sh
cargo add tenonry
```

```rust
use tenonry::Tenonry;

#[tokio::main]
async fn main() -> tenonry::Result<()> {
    let tenonry = Tenonry::new("/absolute/path/to/data-dir")?;
    let report = tenonry.system_report().await?;
    for tool in report.tools {
        println!("{}: {:?} ({:?})", tool.status.name, tool.state, tool.source);
    }
    Ok(())
}
```

主要入口（`Tenonry`）：

| 功能 | API |
| --- | --- |
| 工具目录 | `available_tools`、`is_known_tool` |
| 轻量探测与完整报告 | `probe_report`、`refresh_probe_report`、`system_report`、`selected_report`、`refresh_system_report` |
| 软件源、版本与更新 | `refresh_catalog`、`list_versions`、`check_updates`、`check_update` |
| 操作计划 | `prepare_plan`、`prepare_batch` |
| 启动与执行任务 | `apply_plan`、`start_plan`、`start_request`、`start_batch` |
| 任务记录与复核 | `operations`、`operation`、`latest_operation`、`recheck_operation` |

先生成并审阅操作计划，再将完整计划传回执行接口。执行前重新核对工具与版本状态：若在启动命令前校验失败（如计划过期），`apply_plan` 直接返回对应 `Error`（例如 `ErrorKind::PlanExpired`），异步接口（`start_*`）在任务记录的 `outcome.error` 中保留完整结构化 `Error`。

混合来源（同一工具同时存在包管理器安装与外部可执行文件）、多后端冲突工具及系统受保护组件均禁止自动更新和卸载。

`Tenonry::new` 接收调用方指定的数据目录，构造时不触发磁盘 I/O。任务记录、各后端软件源缓存与跨进程锁分别位于数据目录下的 `operations.json`、`catalog/<backend>/` 和 `tenonry.lock`。克隆实例共享缓存与任务状态；可通过 `with_operation_limits` 注入共享 Tokio 信号量协调并发。

## 独立桌面端

`apps/desktop`（需要 Rust **1.90+**）基于 Tauri 2 与原生 HTML/CSS/ES Modules 构建（无前端打包器与框架依赖），按需启动、单实例运行、关闭主窗口即退出，不常驻托盘或后台守护进程：

- **工具视图**：启动先离线快速探测，随后识别来源与包管理器归属并静默异步检查更新；支持按状态、分类与关键字筛选、单工具详情抽屉、版本选择、单条与跨后端批量计划审阅、实时阶段/日志监控。
- **后端与软件源视图**：展示各平台包管理器可用状态、能力矩阵、托管工具与手动刷新软件源，以及本地数据目录与运行诊断信息。
- **任务记录视图**：持久查看最近操作记录，分开呈现命令执行结果、系统核验结果与结构化错误类别，支持中断/异常任务的只读状态复核。
- **外观与保护**：默认跟随系统浅色/深色主题并支持手动切换；任务执行期间阻止误关主窗口并提示当前进度。

## 构建与检查

```sh
node scripts/check-ui.mjs
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tenonry --no-deps --locked
cargo check --workspace --locked
cargo build --workspace --locked --release
```

直接启动桌面应用：

```sh
cargo run -p tenonry-desktop --release
```

## 贡献、安全与变更记录

- 贡献与架构约定：[CONTRIBUTING.md](CONTRIBUTING.md)
- 安全策略与漏洞反馈：[SECURITY.md](SECURITY.md)
- 版本变更记录：[CHANGELOG.md](CHANGELOG.md)

## 开源协议

本项目采用双协议授权，你可以任选其一使用：

- Apache License, Version 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT License（[LICENSE-MIT](LICENSE-MIT)）
