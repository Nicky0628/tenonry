# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-23

### Added

- Initial public release of the `tenonry` core Rust library (`crates/tenonry`) and `tenonry-desktop` standalone Tauri 2 application (`apps/desktop`).
- Multi-backend package coordination across `APT`, `DNF`, `pacman`, `WinGet`, and `npm` behind a five-layer unidirectional architecture (`model → infra → registry → backend → engine → lib`).
- Offline `PATH` probing, installation ownership attribution, version queries, preflight version-locked plans, per-backend batch transaction merging, task journals (`operations.json`), and post-execution verification.
- Safety guards blocking automatic modification of system-protected components, mixed-source installations, version-manager-managed binaries, and multi-backend conflicts.
- Cross-process file locking powered by standard library `std::fs::File` locks (Rust 1.89+).
- Dual licensing under `MIT OR Apache-2.0`.

[Unreleased]: https://github.com/Nicky0628/tenonry/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Nicky0628/tenonry/releases/tag/v0.1.0
