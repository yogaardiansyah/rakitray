# Rakitray

<div align="center">

**Ultra-Fast, Lightweight, Modular "Install-as-You-Need" Local Development Environment Launcher**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](./LICENSE)
[![CI Status](https://img.shields.io/github/actions/workflow/status/yogaardiansyah/rakitray/ci.yml?branch=main&label=CI)](https://github.com/yogaardiansyah/rakitray/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Core-Rust_2021-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Framework-Tauri_v2-24C8DB.svg?logo=tauri)](https://v2.tauri.app/)
[![SolidJS](https://img.shields.io/badge/UI-SolidJS-2c4f7c.svg?logo=solid)](https://www.solidjs.com/)
[![Bun](https://img.shields.io/badge/Toolchain-Bun-fbf0df.svg?logo=bun&logoColor=black)](https://bun.sh/)
[![Biome](https://img.shields.io/badge/Code_Style-Biome-60a5fa.svg?logo=biome)](https://biomejs.dev/)

*A modern, zero-bloat, cross-platform alternative to XAMPP, Laragon, and FlyEnv.*

[Architecture](./docs/ARCHITECTURE.md) · [Contributing](./CONTRIBUTING.md) · [AI Agent Guidelines](./AGENTS.md) · [Security Policy](./SECURITY.md) · [Legal Notice](./NOTICE)

</div>

---

## Why Rakitray Exists

Local web and backend development shouldn't require sacrificing gigabytes of disk space, hundreds of megabytes of idle RAM, or fighting orphaned background processes:

| Pain Point | Legacy Monoliths (XAMPP / MAMP) | Closed-Source Windows Tools (Laragon) | Electron Wrappers (FlyEnv) | **Rakitray (Rust + Tauri v2 + SolidJS)** |
| :--- | :--- | :--- | :--- | :--- |
| **Disk Footprint** | 800 MB – 2 GB+ pre-bundled bloat | ~300 MB+ bundled installer | ~250 MB+ Chromium bundle | **< 12 MB core binary; 0 MB unused runtimes** |
| **Idle UI Memory** | Legacy Java/WinForms UI | Native Win32 (Windows-only) | 300 MB – 600 MB+ (Chromium) | **~0 MB in Tray Mode / ~25 MB active UI** |
| **Zombie Processes** | Frequent orphaned `mysqld` / `php-cgi` | Occasional lingering workers | Orphaned child PIDs on crash | **Zero zombies (Win32 Job Objects / Unix PGID)** |
| **Platform Support** | Fragmented builds | Windows-only | Cross-platform (heavy) | **First-class Windows, Linux & macOS** |
| **License & Openness** | Open source, archaic architecture | Proprietary / Paid licensing | Open source | **100% Open Source (`AGPL-3.0-only`)** |

**Rakitray** starts as a sub-12 MB native binary containing **zero pre-bundled services**. You download only the exact portable versions of PHP, Nginx, Caddy, MariaDB, PostgreSQL, Redis, Node.js, or Bun that your projects actually use.

---

## Key Features

### 1. Modular "Install-as-You-Need" Binary Catalog
- **Multi-Threaded Chunk Downloader:** Pulls official portable archives directly from upstream mirrors with cryptographic SHA-256 verification and resume support.
- **Side-by-Side Multi-Version Support:** Install PHP `8.1`, `8.2`, `8.3`, and `8.4` or Node `20` and `22` simultaneously without polluting your global system `PATH`.

### 2. Lean Extraction Pipeline (Zero Archive Bloat)
- Upstream database and runtime distributions (especially MariaDB, MySQL, and Windows PHP builds) pack hundreds of megabytes of `.pdb` debug symbols, `mysql-test/` suites, static `.lib` archives, and HTML manuals.
- Rakitray's **Rust Streaming Extractor** filters and strips non-essential files on-the-fly during decompression—reducing extracted disk usage by **up to 75%** without touching disk first.

### 3. Anti-Zombie Process Supervisor
- **Windows:** Assigns every spawned service tree to a dedicated Win32 Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
- **Linux & macOS:** Spawns services in isolated Unix Process Groups (`setpgid`) paired with `PR_SET_PDEATHSIG(SIGTERM)` on Linux and group signal escalation (`SIGTERM` -> grace timeout -> `SIGKILL`).
- Even if Rakitray is force-killed by the OS task manager, **zero orphaned `php-cgi`, `nginx`, or `mariadbd` processes survive**.

### 4. Zero-RAM System Tray Mode
- Closing the dashboard window completely destroys the underlying OS WebView instance (`WebView2` / `WebKitGTK` / `WKWebView`), freeing 100% of UI memory while the lightweight Rust supervisor continues orchestrating your services in the system tray using **< 8 MB of RAM**.

### 5. High-Frequency Raw IPC Telemetry & Log Streaming
- Built on **Tauri v2 Raw IPC Channels (`tauri::ipc::Channel`)** rather than legacy event bus JSON serialization.
- Streams thousands of `stdout`/`stderr` log lines per second and real-time PID CPU/RAM metrics into a virtualized SolidJS log viewer with zero UI frame drops.

### 6. Per-Project Isolation via `.dev.toml`
- Drop a declarative `.dev.toml` file into any project repository to pin exact service versions, custom ports, local domains, and environment variables:
  ```toml
  [project]
  name = "acme-api"
  domain = "acme.test"
  tls = true

  [services]
  php = "8.3.13"
  nginx = "1.26.2"
  mariadb = "11.4.4"
  redis = "7.4.1"

  [env]
  APP_ENV = "local"
  DB_PORT = "3306"
  ```

### 7. Smart Port Doctor
- Pre-flight socket scanner detects port collisions (`80`, `443`, `3306`, `5432`, `6379`, `8080`) before starting a service, identifies the conflicting process name and PID, and offers one-click process termination or port remapping.

### 8. PHP INI & Extension Matrix (v0.2.0)
- **Atomic INI Configuration:** Dual-profile configuration (`DEV` vs `PROD`) with instant worker reload and rollback protection if pre-flight validation fails.
- **Dynamic Module Discovery:** Scans portable `ext/` folders (`.dll` on Windows, `.so` on Linux) and reconciles compiled-in core modules via `php -m`.
- **1-Click Laravel 12/13 Preset:** Automatically enables all core, database, and recommended extensions (`pdo_mysql`, `mbstring`, `curl`, `fileinfo`, `zip`, `bcmath`, `redis`) in one click.
- **Cross-Version Profile Clone:** Synchronize profiles, limits, and extension sets between installed PHP versions while cleanly skipping unavailable modules.
- **Preserved User Directives:** Directives in `<version>-user.ini` are preserved intact and scanned automatically via `PHP_INI_SCAN_DIR`.

---

## Tech Stack

- **Backend / Systems Core:** [Rust](https://www.rust-lang.org/) (Cargo, Tokio async runtime, `reqwest` streaming, `zip`/`tar` stream filtering, `sysinfo`, native Win32/libc process groups) + [Tauri v2](https://v2.tauri.app/).
- **Frontend / Reactive UI:** [SolidJS](https://www.solidjs.com/) + TypeScript + Vite—compiled to direct DOM mutations with **zero Virtual DOM overhead** and custom zero-dependency UI primitives.
- **JavaScript Package Manager & Script Runner:** [Bun](https://bun.sh/).
- **Linter & Formatter:** `cargo fmt` + `cargo clippy` (Rust) and [`@biomejs/biome`](https://biomejs.dev/) (Frontend).

For an in-depth technical breakdown of the IPC design, process supervisor, and module manifest schema, read [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md).

---

## Quick Start for Developers

### Prerequisites
1. **Rust Toolchain** (stable): Install via [rustup.rs](https://rustup.rs/).
2. **Bun** (`>= 1.1`): Install via [bun.sh](https://bun.sh/).
3. **Tauri v2 OS Dependencies:** Follow the [Tauri v2 Prerequisites Guide](https://v2.tauri.app/start/prerequisites/) for your operating system (WebView2 on Windows, `webkit2gtk-4.1` on Linux, Xcode Command Line Tools on macOS).

### Clone & Run in Development Mode
```bash
# 1. Clone the repository
git clone https://github.com/yogaardiansyah/rakitray.git
cd rakitray

# 2. Install frontend dependencies with Bun
bun install

# 3. Launch Rakitray in hot-reload development mode
bun run tauri dev
```

### Build Production Installer
```bash
# Compiles optimized Rust release binary + SolidJS bundle and packages OS installers
bun run tauri build
```

### Code Quality & Linting
```bash
# Frontend formatting & lint check (Biome)
bunx biome check .

# Rust formatting & strict Clippy lints
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

---

## Contributing

We welcome contributions from systems engineers, frontend developers, and runtime package maintainers!
- Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) for environment setup, pull request gates, and how to add new downloadable modules.
- Using AI coding assistants (Cursor, Claude Code, Antigravity, GitHub Copilot, Windsurf)? Make sure your agent reads [`AGENTS.md`](./AGENTS.md) first so it respects Rakitray's strict Rust/SolidJS architectural boundaries and licensing rules.

---

## License & Mandatory Attribution (`AGPL-3.0-only`)

Copyright (C) 2026 **Yoga ardiansyah** ([@yogaardiansyah](https://github.com/yogaardiansyah)).

**Rakitray** is free and open-source software licensed under the **GNU Affero General Public License v3.0 only (`AGPL-3.0-only`)** with strict **Section 7(b) and Section 5(d) Attribution & Legal Notice Preservation Requirements**.

Pursuant to [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE), any fork, modified version, binary redistribution, or network/cloud deployment of **Rakitray** **MUST**:
1. Retain both the [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE) files intact and unmodified in the root directory.
2. Clearly state in documentation and release materials that the work is derived from **Rakitray** by **Yoga ardiansyah** with a clickable link back to [`https://github.com/yogaardiansyah/rakitray`](https://github.com/yogaardiansyah/rakitray).
3. Preserve the `"Powered by Rakitray — Created by Yoga ardiansyah"` (or `"Based on Rakitray by Yoga ardiansyah"`) attribution notice in the application's interactive UI (About dialog, status footer, system tray menu) and CLI output (`--version` / `--help`).

See [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE) for full legal terms.

