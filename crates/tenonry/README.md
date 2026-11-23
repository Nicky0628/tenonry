# tenonry

[![Crates.io](https://img.shields.io/crates/v/tenonry.svg)](https://crates.io/crates/tenonry)
[![Documentation](https://docs.rs/tenonry/badge.svg)](https://docs.rs/tenonry)
[![License](https://img.shields.io/crates/l/tenonry.svg)](https://github.com/Nicky0628/tenonry#license)

Cross-platform development tool discovery, multi-backend package coordination, and verifiable task execution.

Built on top of existing system and ecosystem package managers (`APT`, `DNF`, `pacman`, `WinGet`, and `npm`), `tenonry` provides offline `PATH` probing, installation ownership attribution, version queries, preflight plan generation, multi-backend batch orchestration, task journals, and post-execution verification.

## Features

- **Multi-backend coordination**: Supports `APT`, `DNF`, `pacman`, `WinGet`, and `npm` behind a unified async API.
- **Safety boundaries**: Strictly respects installation ownership; refuses automatic modification of system-protected components, mixed-source installs, and multi-backend conflicts.
- **Plan-then-apply workflow**: Generates version-locked plans before execution and re-verifies live system state prior to running commands.
- **Verifiable task journal**: Persists operations (`operations.json`), backend catalog caches (`catalog/`), and a cross-process advisory lock (`tenonry.lock`) under a caller-supplied data directory.

## Supported Backends

| Backend | Scope | Platform | Install / Update / Uninstall | Version Selection | Batch Transactions |
| --- | --- | --- | --- | --- | --- |
| APT | System | Linux | Yes | Yes | Yes |
| DNF | System | Linux | Yes | — | Yes |
| pacman | System | Linux | Yes | — | Yes |
| WinGet | System | Windows | Yes | — | Yes |
| npm | Ecosystem | Linux, Windows | Yes | Yes | — |

## Quick Start

Requires Rust **1.89+** (Edition 2024).

```toml
[dependencies]
tenonry = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust,no_run
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

> **Note on user-facing messages**: Structured categories (`ErrorKind`, `ToolState`, `ToolSource`, etc.) are language-neutral enums, while default human-readable status and error messages are currently authored in Simplified Chinese.

## Documentation & Source

- API Documentation: <https://docs.rs/tenonry>
- Repository & Desktop App: <https://github.com/Nicky0628/tenonry>
- Changelog: <https://github.com/Nicky0628/tenonry/blob/main/CHANGELOG.md>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/Nicky0628/tenonry/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/Nicky0628/tenonry/blob/main/LICENSE-MIT))

at your option.
