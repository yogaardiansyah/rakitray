# AI Coding Agent Instructions (`AGENTS.md`)

Welcome, AI Coding Assistant (Claude Code, Google Antigravity, Cursor, GitHub Copilot, OpenAI Codex, Windsurf, Cline, etc.).

This file is the **authoritative operating manual** for any AI agent reading, modifying, or generating code in the **Rakitray** repository (`https://github.com/yogaardiansyah/rakitray`). You **MUST** obey every rule in this document before proposing or applying changes.

---

## 1. Project Identity & Core Philosophy

- **Project Name:** Rakitray
- **Author & Copyright Holder:** Yoga ardiansyah (`@yogaardiansyah`)
- **Repository:** `https://github.com/yogaardiansyah/rakitray`
- **License:** `AGPL-3.0-only` with strict **Section 7(b) & Section 5(d)** attribution preservation enforced via [`NOTICE`](./NOTICE).
- **Mission:** An ultra-fast, lightweight, modular "install-as-you-need" Local Development Environment Launcher built with **Rust + Tauri v2 + SolidJS + Bun**, replacing bloated tools like XAMPP, Laragon, and FlyEnv.

---

## 2. Hard Architectural Boundaries (Non-Negotiable)

Read [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) before designing features. Rakitray enforces a strict split between `src/` (Frontend UI) and `src-tauri/` (Systems Core):

### 2.1 `src-tauri/` (Rust Systems Core — All Business & OS Logic)
All heavy lifting, I/O, networking, and OS interaction **MUST** live in Rust:
- **Process Supervision:** Spawn and manage child services (Nginx, Caddy, PHP-CGI, MariaDB, PostgreSQL, Redis, Node, Bun) exclusively in Rust.
  - **Windows:** Every spawned child process MUST be assigned to a Win32 Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so zero orphaned/zombie processes survive if Rakitray exits or crashes.
  - **Unix (Linux/macOS):** Every spawned child process MUST be placed in an isolated Process Group (`process_group(0)` / `setpgid(0, 0)`) and on Linux configured with `prctl(PR_SET_PDEATHSIG, SIGTERM)`. Stopping a service signals the entire negative PGID (`-pgid`) and escalates from `SIGTERM` to `SIGKILL` after the timeout.
  - **Security:** Never spawn processes via shell string interpolation (`sh -c` or `cmd /C`). Always pass explicit argument arrays to `Command::new(binary_path).args(...)`.
- **Binary Downloading & Lean Extraction:**
  - Multi-threaded downloading, SHA-256 verification, and archive extraction happen in Rust.
  - Stream-filter archives during extraction to strip bloat (`**/*.pdb`, `**/*.lib`, `mysql-test/**`, `sql-bench/**`, docs) before writing to disk.
  - Enforce strict **Zip Slip / Path Traversal** validation on every archive entry.
- **Config Templating & `.dev.toml` Isolation:**
  - Parse `.dev.toml` and render `nginx.conf`, `Caddyfile`, `php.ini`, and `my.cnf` in Rust.
- **Zero-RAM Tray Mode:**
  - Closing the main window destroys the WebView instance (`window.destroy()`) to drop UI memory usage to `0 MB` while the Rust Tokio supervisor keeps running in the system tray.

### 2.2 `src/` (SolidJS + TypeScript — Pure Reactive UI Only)
- **Framework:** SolidJS (`solid-js`). Remember that SolidJS components run **once** on mount—never destructure props at the top level of a component (use `props.foo` or `splitProps`), and use fine-grained signals/stores (`createSignal`, `createStore`, `createMemo`, `<For>`, `<Show>`).
- **Zero Reactisms:** Do **NOT** write React hooks (`useState`, `useEffect`, `useMemo`, `useCallback`), do **NOT** use Virtual DOM patterns, and do **NOT** install React libraries.
- **No System Logic in JS:** Do **NOT** use `@tauri-apps/plugin-fs` or `@tauri-apps/plugin-shell` in `src/`. Do **NOT** parse config files or poll process status with `setInterval` in TypeScript.
- **High-Frequency IPC via `Channel`:**
  - Real-time `stdout`/`stderr` log streams and PID telemetry (CPU%, RSS RAM, state transitions) MUST use Tauri v2's `Channel` API (`new Channel<T>()` in `@tauri-apps/api/core` paired with `tauri::ipc::Channel` in Rust)—never global `emit()`/`listen()` JSON event broadcasts.

---

## 3. Package Manager & Tooling Commands

- **Use `bun` exclusively** for JavaScript/TypeScript tasks. Never run or generate lockfiles for `npm`, `pnpm`, or `yarn`.
- Before finishing any task, verify your changes with these exact commands:

```bash
# 1. Frontend format & lint check (Biome)
bunx biome check .

# 2. Auto-fix Frontend formatting/lints if needed
bunx biome check --write .

# 3. Rust formatting check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check

# 4. Rust strict Clippy check (zero warnings allowed)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# 5. Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 4. Coding Style & Quality Standards

- **Rust (`src-tauri/`):**
  - 4-space indentation, `edition = "2021"` (or later).
  - Never use `.unwrap()` or `.expect()` on fallible I/O, network, IPC, or OS process operations in production paths—define typed errors using `thiserror` and return `Result<T, RakitrayError>`.
  - Keep async tasks non-blocking; offload CPU-heavy decompression or synchronous Win32/libc calls to `tokio::task::spawn_blocking`.
- **TypeScript / SolidJS (`src/`):**
  - 2-space indentation, strict TypeScript (`noImplicitAny`, `strictNullChecks`, no `any` casts).
  - Enforced by [`biome.json`](./biome.json).
  - Build lightweight, accessible, zero-overhead custom UI components rather than pulling in bloated UI dependencies.

---

## 5. Legal & Attribution Protection Guardrails (`AGPL-3.0-only` + `NOTICE`)

As an AI agent, you are **strictly prohibited** from:
1. Deleting, truncating, or weakening [`LICENSE`](./LICENSE) or [`NOTICE`](./NOTICE).
2. Removing or hiding the mandatory Section 7(b) / Section 5(d) attribution notices (`"Powered by Rakitray — Created by Yoga ardiansyah"` and the link to `https://github.com/yogaardiansyah/rakitray`) from the application UI (About dialog, footer, system tray menu) or CLI output (`--version` / `--help`).
3. Introducing proprietary or AGPLv3-incompatible dependencies into the core repository.

---

## 6. Always-On Assistant Modes (Caveman, Ponytail, RTK)

For this workspace, the following operational modes are permanently active:
- **Caveman Mode (Always-On):** Respond terse like smart caveman. Answer first. Zero conversational ceremony, greetings, hedging, or fluff. All technical substance, code, file paths, commands, and numbers stay 100% exact.
- **Ponytail Mode (Always-On):** Lazy senior dev philosophy. Smallest complete change that solves whole task. No premature abstraction, no bloat, standard library/existing helpers first, no unrequested features.
- **RTK Optimization (Always-On):** Output token efficiency and tool execution optimizations via RTK.

