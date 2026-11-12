## Summary

<!-- Briefly describe what this Pull Request accomplishes and why. -->

Closes #<!-- issue-number -->

---

## Type of Change

- [ ] **Bug Fix** (non-breaking change that resolves an issue)
- [ ] **New Feature** (non-breaking change that adds functionality)
- [ ] **New Runtime Module** (addition or version update in the Install-as-You-Need catalog)
- [ ] **Performance / Memory Optimization** (improves IPC throughput, RAM footprint, or extraction speed)
- [ ] **Documentation / Governance / CI**

---

## Architectural & Quality Checklist

Before requesting a review, verify that all of the following requirements are met:

### 1. Strict Architectural Boundary (`docs/ARCHITECTURE.md` & `AGENTS.md`)
- [ ] **System Logic in Rust (`src-tauri/`):** All process management, port scanning, archive extraction, `.dev.toml` parsing, and file I/O are implemented exclusively in Rust.
- [ ] **Pure Reactive UI in SolidJS (`src/`):** Frontend changes contain only fine-grained SolidJS signals/stores, UI rendering, and typed IPC calls (no React hooks, no VDOM libraries, no `setInterval` status polling).
- [ ] **High-Frequency IPC:** Any streaming log or process telemetry uses `tauri::ipc::Channel` rather than global JSON event broadcasts (`app.emit`).

### 2. Process Supervision & Security Safety
- [ ] **No Zombie Processes Introduced:** Any spawned child process is bound to a Win32 Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) on Windows and an isolated Unix Process Group (`setpgid` + `PR_SET_PDEATHSIG`) on Linux/macOS.
- [ ] **Safe Execution & Extraction:** No shell string interpolation (`sh -c` / `cmd /C`) is used for process spawning, and archive extraction paths are validated against Zip Slip traversal.

### 3. Automated Linting, Formatting & Tests
- [ ] `bunx biome check .` passes with **0 errors and 0 warnings**.
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passes cleanly.
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes with **0 warnings**.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes on all unit/integration tests.

### 4. License & Mandatory Attribution (`AGPL-3.0-only` + `NOTICE`)
- [ ] All [`LICENSE`](../LICENSE) and [`NOTICE`](../NOTICE) headers, copyright attributions to **Yoga ardiansyah (@yogaardiansyah)**, and interactive UI/CLI attribution notices remain intact.

---

## Verification & Screenshots

<!-- Describe how you tested these changes locally (OS tested, commands run, and any UI screenshots/benchmarks if applicable). -->

