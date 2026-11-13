# Rakitray

Modular local development environment supervisor built with Rust, Tauri v2, and SolidJS.

[![License: AGPL-3.0-only](https://img.shields.io/badge/License-AGPL_3.0--only-525866.svg)](./LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/yogaardiansyah/rakitray/ci.yml?branch=main&label=CI)](https://github.com/yogaardiansyah/rakitray/actions/workflows/ci.yml)

[Architecture](./docs/ARCHITECTURE.md) · [Contributing](./CONTRIBUTING.md) · [Agent Guidelines](./AGENTS.md) · [Security](./SECURITY.md) · [Notice](./NOTICE)

---

## Overview

Rakitray separates system orchestration from the user interface:

- **Rust Core (`crates/rakit-core`, `src-tauri`)** handles process supervision, archive downloading and extraction, `.dev.toml` configuration rendering, and telemetry streaming.
- **SolidJS UI (`src`)** renders the workstation interface over Tauri v2 IPC (`tauri::ipc::Channel`) without running system or filesystem logic in JavaScript.
- **CLI (`crates/rakit-cli`)** provides headless control over the same core engine.

No runtimes are bundled in the application binary. Services are installed on demand into the local `runtime/bin/` directory without modifying the system `PATH`.

---

## Capabilities

- **Process Supervision**: Runs Nginx, Caddy, PHP-CGI/FPM, MariaDB, PostgreSQL, Redis, and Node/Bun in isolated process trees—Win32 Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) on Windows, and process groups (`setpgid`) with `PR_SET_PDEATHSIG(SIGTERM)` on Linux.
- **RakiStore Package Catalog**: Downloads portable runtime archives with SHA-256 verification and streaming archive filtering (skipping debug symbols, test suites, and documentation during extraction). Supports 1-click stack templates.
- **PHP INI & Extension Matrix**: Manages dual `DEV` / `PROD` INI profiles per PHP version, discovers extensions from `ext/` and `php -m`, applies Laravel presets, and clones profiles across installed PHP versions.
- **Sites & Routing**: Maps local domains, document roots, and ports to supervised web servers and PHP upstreams.
- **Live Telemetry & Logs**: Streams process status, PID, CPU/RAM usage, and `stdout`/`stderr` batches over `tauri::ipc::Channel`.
- **Tray Mode**: Closing the main window destroys the WebView while the Rust supervisor remains active in the system tray.

---

## Repository Structure

```text
rakitray/
├── src/                # SolidJS + TypeScript UI
├── src-tauri/          # Tauri v2 application host and IPC commands
├── crates/
│   ├── rakit-core/     # Supervisor, store downloader/extractor, drivers, PHP matrix
│   └── rakit-cli/      # CLI frontend for rakit-core
└── docs/               # Architecture and design documents
```

---

## Development

### Prerequisites

- **Rust** (stable via [rustup.rs](https://rustup.rs/))
- **Bun** (`>= 1.1` via [bun.sh](https://bun.sh/))
- **Tauri v2 OS dependencies**: see the [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/). On Debian/Ubuntu, ensure `libwebkit2gtk-4.1-dev` and `libgbm-dev` are installed.

### Setup & Run

```bash
git clone https://github.com/yogaardiansyah/rakitray.git
cd rakitray

bun install
bun run tauri dev
```

### Build

```bash
bun run tauri build
```

### Verification

```bash
bunx biome check .
bun run typecheck
bun test

cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for pull request guidelines and module catalog conventions, and [`AGENTS.md`](./AGENTS.md) for architectural boundaries.

---

## License & Attribution

Copyright (C) 2026 Yoga ardiansyah (`@yogaardiansyah`).

Licensed under [`AGPL-3.0-only`](./LICENSE) with Section 7(b) and Section 5(d) attribution requirements enforced via [`NOTICE`](./NOTICE). Forks, modified versions, binary redistributions, and network deployments must:

1. Retain [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE) unmodified.
2. Credit **Rakitray** by **Yoga ardiansyah** with a link to `https://github.com/yogaardiansyah/rakitray` in documentation and release materials.
3. Preserve `"Powered by Rakitray — Created by Yoga ardiansyah"` (or `"Based on Rakitray by Yoga ardiansyah"`) in the application UI and CLI `--version` / `--help` output.
