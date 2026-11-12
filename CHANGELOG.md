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

## [0.2.0] - 2026-11-12

### Added
- **PHP INI & Extension Matrix (`src-tauri/src/discovery/php_ext.rs`, `src-tauri/src/drivers/php_fpm.rs`):**
  - Cross-platform extension scanner for Windows `.dll` and Linux `.so` with automatic built-in module reconciliation via `php -n -m`.
  - Dual-profile INI generator (`DEV` vs `PROD`) with distinct limit profiles, error display controls, and topological dependency sorting (`mbstring` before `exif`, `pdo` before `pdo_mysql`).
  - Atomic INI writes with `<version>-generated.ini.tmp` sync, syntax validation pre-flight via `<php-bin> -n -c <tmp> -m`, `.bak` backup, and automatic rollback on reload failure.
  - User customization preservation: user directives in `<version>-user.ini` remain untouched and are loaded through `PHP_INI_SCAN_DIR`.
  - Per-version concurrency protection via Tokio Mutex in `PhpMatrixStore`.
  - Debounced supervisor worker reloading with `PhpMatrixEvent` channel dispatch.
- **Tauri IPC Commands (`src-tauri/src/commands/php_matrix.rs`):**
  - `subscribe_php_matrix`: Real-time streaming channel for matrix reload and rollback events.
  - `get_php_matrix`: Returns version, profile, limits, categorized extension list, and compatibility warnings.
  - `toggle_php_extension`: Safe toggling validated against scanned allowlist, with dependency enforcement and hostile input rejection.
  - `apply_php_profile`: Staging and applying DEV/PROD profiles with custom INI limits.
  - `apply_laravel_preset`: 1-Click activation of Laravel 12/13 core and recommended extensions.
  - `clone_php_matrix`: Cross-version profile cloning with diff analysis and ext-dir path isolation.
- **SolidJS Reactive Workstation UI (`src/components/`):**
  - Atomic Design components: `ExtensionCheckbox`, `ProfileBadge`, `ProfileToggle`, `LimitField`, and `PendingBar`.
  - Organism `PhpMatrixPanel` with dynamic runtime selector, 5 categorized extension tiers, clone diff confirmation dialog, and status banners.
  - Sidebar navigation tab `"PHP Matrix"` and shell routing in `WorkstationShell`.
- **Testing & CI:**
  - Dedicated end-to-end fake PHP runtime integration test suite (`src-tauri/tests/php_matrix_e2e.rs`).
  - Added frontend TypeScript type check (`tsc --noEmit`) to CI workflow.

### Changed
- Bumped application version to `0.2.0` across `package.json`, `Cargo.toml`, and `tauri.conf.json`.
- Extended `ActiveTab` in `supervisorStore.ts` to include `"phpMatrix"`.

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

[Unreleased]: https://github.com/yogaardiansyah/rakitray/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/yogaardiansyah/rakitray/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yogaardiansyah/rakitray/releases/tag/v0.1.0

