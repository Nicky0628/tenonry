# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-27

### Fixed

- **npm cache revalidation**: Removed `--prefer-offline` from `npm` preflight and install commands so stale abbreviated packuments in `~/.npm/_cacache` no longer fail with `ETARGET` on newly published releases, with a single `--prefer-online` dry-run retry when a recently cached entry within `max-age` is missing the target version.
- **npm error classification & resilience**: Parsed structured JSON error payloads (and npm 9/10/11 stderr error codes) on non-zero exits to classify `EBADENGINE`/`EBADPLATFORM`, `ETARGET`/`E404`, auth/permission, dependency conflict, and network errors accurately instead of misreporting every preflight failure as a Node.js/platform mismatch; kept fast-fail fetch flags (`--fetch-retries=0`, `--fetch-timeout=12000`) restricted to read-only probes rather than real `npm install` executions.
- **APT version-selection & inventory parsing**: Added the missing `authorization_issue()` check in `apt::versions::plan_version`, and switched `dpkg-query` parsing in `AptBackend::detect()` to tab-delimited columns so not-installed entries with empty versions do not break APT detection.
- **Cross-platform path & WinGet list parsing**: Normalized trailing backslashes in Windows path comparisons, allowed symlinked directories when verifying `npm`'s global `bin` directory on `PATH`, and updated WinGet list parsing to handle `< <version>` columns and reject truncated (`…`) upgrade versions.
- **Desktop error summary patterns**: Removed WinGet `0x8A15002B` (`UPDATE_NOT_APPLICABLE`) from the cancelled-authorization pattern and added `ETARGET` to the missing package/version pattern.

## [0.1.0] - 2026-11-23

### Added

- Initial public release of the `tenonry` core Rust library (`crates/tenonry`) and `tenonry-desktop` standalone Tauri 2 application (`apps/desktop`).
- Multi-backend package coordination across `APT`, `DNF`, `pacman`, `WinGet`, and `npm` behind a five-layer unidirectional architecture (`model → infra → registry → backend → engine → lib`).
- Offline `PATH` probing, installation ownership attribution, version queries, preflight version-locked plans, per-backend batch transaction merging, task journals (`operations.json`), and post-execution verification.
- Safety guards blocking automatic modification of system-protected components, mixed-source installations, version-manager-managed binaries, and multi-backend conflicts.
- Cross-process file locking powered by standard library `std::fs::File` locks (Rust 1.89+).
- Dual licensing under `MIT OR Apache-2.0`.

[Unreleased]: https://github.com/Nicky0628/tenonry/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/Nicky0628/tenonry/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Nicky0628/tenonry/releases/tag/v0.1.0
