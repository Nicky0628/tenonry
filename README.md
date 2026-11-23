<div align="center">

# Tenonry

English | [简体中文](README.zh-CN.md)

[![CI](https://github.com/Nicky0628/tenonry/actions/workflows/check.yml/badge.svg)](https://github.com/Nicky0628/tenonry/actions/workflows/check.yml)
[![Crates.io](https://img.shields.io/crates/v/tenonry.svg)](https://crates.io/crates/tenonry)
[![Documentation](https://docs.rs/tenonry/badge.svg)](https://docs.rs/tenonry)
[![License](https://img.shields.io/crates/l/tenonry.svg)](#license)

Cross-platform development tool environment coordination library and standalone desktop manager.

</div>

Built on top of existing system and ecosystem package managers in a single Cargo Workspace, Tenonry provides tool probing, installation ownership attribution, version queries, preflight plans, multi-backend batch orchestration, task journals, and post-execution verification.

## Supported Scope

| Backend | Scope | Platform | Install / Update / Uninstall | Version Selection | Batch Transactions |
| --- | --- | --- | --- | --- | --- |
| APT | System | Linux | Yes | Yes | Yes |
| DNF | System | Linux | Yes | — | Yes |
| pacman | System | Linux | Yes | — | Yes |
| WinGet | System | Windows | Yes | — | Yes |
| npm | Ecosystem | Linux, Windows | Yes | Yes | — |

Available tools and versions are determined by the built-in tool registry, locally available backends, and configured package sources. Operations strictly respect each tool's actual installation source; system-protected components, mixed-source installs, and multi-backend conflicts are blocked from automatic modification.

> **Platform note**: Core library compilation is verified on Linux, Windows, and macOS. Native package backends currently target Linux (`APT`, `DNF`, `pacman`, `npm`) and Windows (`WinGet`, `npm`).

## Workspace Layout

```text
tenonry/
├── Cargo.toml                  # Workspace root manifest
├── crates/
│   └── tenonry/                # Core Rust library (zero UI dependencies, published on crates.io)
│       └── src/
│           ├── lib.rs          # Public Tenonry facade and re-exports
│           ├── error.rs        # Structured Error / ErrorKind
│           ├── model/          # Strongly typed domain models & serde DTOs
│           ├── infra/          # Processes, paths, probing, file locks, elevation, atomic storage
│           ├── registry/       # Backend-agnostic tool catalog and shared rules
│           ├── backend/        # BackendDriver contract & vertical per-backend implementations
│           └── engine/         # Probing, updates, versions, planning, execution, verification, journals
├── apps/
│   └── desktop/                # Standalone Tauri 2 desktop app (vanilla HTML/CSS/ES Modules)
│       ├── src/                # Single-instance guard, origin checks, IPC adapter, close guard
│       └── ui/                 # Tools, backends & sources, task history, and plan/monitor dialogs
└── scripts/
    └── check-ui.mjs            # ES module syntax and static asset reference checker
```

## Using the `tenonry` Crate

Requires Rust **1.89+** (Edition 2024). Executing package operations requires the corresponding host package manager to be installed.

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

Primary entry points on `Tenonry`:

| Capability | API |
| --- | --- |
| Tool catalog | `available_tools`, `is_known_tool` |
| Lightweight probe & full inventory | `probe_report`, `refresh_probe_report`, `system_report`, `selected_report`, `refresh_system_report` |
| Catalogs, versions & updates | `refresh_catalog`, `list_versions`, `check_updates`, `check_update` |
| Preflight planning | `prepare_plan`, `prepare_batch` |
| Execution & async tasks | `apply_plan`, `start_plan`, `start_request`, `start_batch` |
| Task history & re-verification | `operations`, `operation`, `latest_operation`, `recheck_operation` |

Always generate and review an operation plan first, then pass the full plan back to an execution method. Live tool and version state is re-checked before running commands: if validation fails prior to launching commands (for example, `ErrorKind::PlanExpired`), `apply_plan` returns the structured `Error` directly while async methods (`start_*`) preserve it in `outcome.error`.

`Tenonry::new` accepts a caller-supplied data directory and performs no disk I/O during construction. Task journals, per-backend catalog caches, and the cross-process advisory lock live under `<data-dir>/operations.json`, `<data-dir>/catalog/<backend>/`, and `<data-dir>/tenonry.lock`. Clones share in-memory caches and active task state, and `with_operation_limits` allows injecting shared Tokio semaphores to coordinate concurrency.

> **Note on user-facing messages**: Structured categories (`ErrorKind`, `ToolState`, `ToolSource`, etc.) are language-neutral enums, while default human-readable status and error strings are currently authored in Simplified Chinese.

## Standalone Desktop App

`apps/desktop` (requires Rust **1.90+**) is built with Tauri 2 and vanilla HTML/CSS/ES Modules (no frontend bundler or framework dependencies). It runs on demand as a single instance and exits cleanly when the main window closes, without a background tray or daemon:

- **Tools view**: Performs an offline fast probe on startup, then resolves backend ownership and checks updates asynchronously; supports status/category/keyword filters, detail drawer, version picker, single & cross-backend batch plan review, and live phase/log monitoring.
- **Backends & sources view**: Displays backend availability, capability matrix, managed tools, manual catalog refresh, local data directory, and runtime diagnostics.
- **Task history view**: Persists recent operations with separate command execution status, live verification status, and structured error kinds, plus read-only re-verification for interrupted tasks.
- **Appearance & safeguards**: Follows system light/dark theme by default with manual override, and prevents accidental window closure while a task is actively running.

## Building & Checking

```sh
node scripts/check-ui.mjs
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tenonry --no-deps --locked
cargo check --workspace --locked
cargo build --workspace --locked --release
```

Run the desktop app directly:

```sh
cargo run -p tenonry-desktop --release
```

## Contributing, Security & Changelog

- Contribution & architecture guidelines: [CONTRIBUTING.md](CONTRIBUTING.md)
- Security policy & vulnerability reporting: [SECURITY.md](SECURITY.md)
- Release history: [CHANGELOG.md](CHANGELOG.md)

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work shall be dual-licensed as above, without any additional terms or conditions.
