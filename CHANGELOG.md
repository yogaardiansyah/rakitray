# Changelog

All notable changes to **Rakitray** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial open-source repository governance, legal attribution protections (`LICENSE` + `NOTICE` enforcing `AGPL-3.0-only` Section 7(b) and Section 5(d)), and contributor documentation.
- System architecture specification (`docs/ARCHITECTURE.md`) detailing strict `src/` (SolidJS) vs. `src-tauri/` (Rust) boundaries, High-Frequency `tauri::ipc::Channel` telemetry/log streaming, Anti-Zombie Process Supervision (Win32 Job Objects & Unix Process Groups), Lean Binary Extraction filter pipeline, and Module Manifest schema.
- AI coding assistant instruction manual (`AGENTS.md`) to enforce architectural, performance, and legal compliance for AI-assisted contributions.
- GitHub Issue Forms (`bug_report.yml`, `feature_request.yml`, `module_request.yml`) and Pull Request checklist template (`PULL_REQUEST_TEMPLATE.md`).
- Multi-platform GitHub Actions CI (`ci.yml`) and automated release build workflows (`release.yml`) for Tauri v2 + Bun + SolidJS + Rust.
- Workspace tooling configurations: `.editorconfig`, `.gitignore`, and `biome.json`.

---

## [0.1.0] - 2026-11-12 — *Unreleased (Initial Architecture Setup)*

### Planned for `v0.1.0` Milestone
- **Rust Core (`src-tauri/`):**
  - Anti-Zombie Process Supervisor using Win32 Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) on Windows and isolated Process Groups (`setpgid` + `PR_SET_PDEATHSIG`) on Linux/macOS.
  - Multi-threaded HTTP downloader with SHA-256 verification and streaming archive extractor stripping `.pdb`, `.lib`, and test suites on the fly.
  - Declarative `.dev.toml` per-project runtime isolation and dynamic config template engine (`nginx.conf`, `Caddyfile`, `php.ini`, `my.ini`).
  - Smart Port Doctor for pre-flight TCP/UDP port conflict detection and PID resolution.
  - Zero-RAM System Tray lifecycle controller that destroys the WebView window when minimized to tray.
- **SolidJS UI (`src/`):**
  - Zero-overhead reactive dashboard, virtualized 60fps log viewer backed by `tauri::ipc::Channel`, module catalog manager, and interactive Port Doctor dialog.

[Unreleased]: https://github.com/yogaardiansyah/rakitray/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yogaardiansyah/rakitray/releases/tag/v0.1.0

