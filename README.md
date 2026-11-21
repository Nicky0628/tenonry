# Tenon

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

## 工作区结构

```text
tenon/
├── Cargo.toml                  # Workspace 根配置
├── crates/
│   └── tenon/                  # 核心 Rust 库（零 UI 依赖）
│       └── src/
│           ├── lib.rs          # 对外门面 Tenon 与公开导出
│           ├── error.rs        # 结构化错误 Error / ErrorKind
│           ├── model/          # 强类型领域模型与序列化 DTO
│           ├── infra/          # 进程、路径、探测、锁、提权、原子读写
│           ├── registry/       # 与包管理器无关的工具目录与通用规则
│           ├── backend/        # BackendDriver 契约与各后端纵向实现
│           └── engine/         # 探测、更新、版本、计划、执行、核验与任务日志
└── apps/                       # 预留给独立桌面 GUI
```

## 使用

需要支持 Rust 2024 edition 的稳定工具链。执行包管理操作时，本机需已安装对应的包管理器。

```rust
use tenon::Tenon;

#[tokio::main]
async fn main() -> tenon::Result<()> {
    let tenon = Tenon::new("/absolute/path/to/data-dir")?;
    let report = tenon.system_report().await?;
    for tool in report.tools {
        println!("{}: {:?} ({:?})", tool.status.name, tool.state, tool.source);
    }
    Ok(())
}
```

主要入口（`Tenon`）：

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

`Tenon::new` 接收调用方指定的数据目录，构造时不触发磁盘 I/O。任务记录、各后端软件源缓存与跨进程锁分别位于数据目录下的 `operations.json`、`catalog/<backend>/` 和 `tenon.lock`。克隆实例共享缓存与任务状态；可通过 `with_operation_limits` 注入共享 Tokio 信号量协调并发。

## 构建与检查

```sh
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo check --workspace --locked
cargo build --workspace --locked --release
```

贡献与架构约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。
