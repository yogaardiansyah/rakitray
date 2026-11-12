# Contributing to Rakitray

Thank you for your interest in contributing to **Rakitray**! Our goal is to build the fastest, leanest, and most reliable open-source local development environment launcher across Windows, Linux, and macOS.

Before opening an issue or submitting a Pull Request, please read this guide and [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) carefully.

> [!IMPORTANT]
> **Using an AI Coding Assistant?**
> If you are contributing with the help of an AI agent (such as Cursor, Claude Code, Google Antigravity, GitHub Copilot, Codex, or Windsurf), make sure your tool loads [`AGENTS.md`](./AGENTS.md) so all generated code strictly complies with Rakitray's architectural boundaries and `AGPL-3.0-only` + `NOTICE` attribution requirements.

---

## 1. Prerequisites

To build and test Rakitray locally, you need:

1. **Rust Toolchain (Stable):**
   Install via [rustup](https://rustup.rs/):
   ```bash
   rustup toolchain install stable
   rustup component add rustfmt clippy
   ```
2. **Bun (`>= 1.1`):**
   Rakitray uses `bun` exclusively for frontend dependency management and script execution (do **not** commit `package-lock.json` or `yarn.lock`).
   ```bash
   curl -fsSL https://bun.sh/install | bash
   ```
3. **OS Build Dependencies for Tauri v2:**
   - **Ubuntu / Debian:**
     ```bash
     sudo apt-get update
     sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
       libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf pkg-config
     ```
   - **Windows:**
     - Microsoft Visual Studio C++ Build Tools (Desktop development with C++)
     - WebView2 Runtime (pre-installed on Windows 10/11)
   - **macOS:**
     - Xcode Command Line Tools (`xcode-select --install`)

---

## 2. Development Workflow

1. **Fork and Clone the Repository:**
   ```bash
   git clone https://github.com/<your-username>/rakitray.git
   cd rakitray
   ```

2. **Install Frontend Dependencies:**
   ```bash
   bun install
   ```

3. **Run in Development Mode:**
   ```bash
   bun run tauri dev
   ```
   This starts the Vite + SolidJS dev server and compiles the Rust `src-tauri/` core with hot-reloading enabled.

4. **Create a Feature or Fix Branch:**
   Use descriptive branch prefixes:
   - `feat/<short-slug>` — New capabilities or runtime modules
   - `fix/<short-slug>` — Bug fixes in supervisor, extractor, or UI
   - `perf/<short-slug>` — Memory, startup, or IPC throughput optimizations
   - `docs/<short-slug>` — Documentation and governance updates

---

## 3. Architectural Rules to Respect

Every Pull Request is reviewed against the boundaries defined in [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md):

1. **All System & Business Logic Belongs in Rust (`src-tauri/`):**
   Process spawning, port scanning, archive extraction, `.dev.toml` parsing, and config file templating **must** be implemented in Rust.
2. **Keep `src/` (SolidJS) Strictly for Reactive UI:**
   Use fine-grained SolidJS signals/stores and zero-overhead custom components. Do not introduce React compatibility layers, Virtual DOM libraries, or heavy third-party component kits.
3. **Zero Zombie Processes:**
   Any code that spawns child processes must attach them to the platform process supervisor (Win32 Job Objects on Windows, isolated Process Groups `setpgid` on Unix) and include cleanup tests.
4. **High-Frequency Streams Use `tauri::ipc::Channel`:**
   Never stream logs or real-time process metrics over global `app.emit()` JSON events.

---

## 4. Mandatory Code Quality Gates

Before opening a Pull Request, all of the following checks **must pass locally with zero warnings or errors** (our GitHub Actions CI pipeline enforces the exact same commands):

### Frontend (Biome)
```bash
# Check formatting, linting, and import sorting
bunx biome check .

# Auto-fix formatting and safe lint issues
bunx biome check --write .
```

### Backend / Systems Core (Rust)
```bash
# Verify Rust formatting
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check

# Run strict Clippy lints (warnings are treated as errors)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Run Rust unit and integration tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 5. How to Propose or Add a New Runtime Module

Rakitray's "Install-as-You-Need" catalog is designed to make adding new portable runtimes (e.g., Caddy, FrankenPHP, PostgreSQL, Valkey, Bun, Go, Mailpit) clean and declarative:

1. **Open a Module Request Issue First:**
   Use the [Module Request Issue Template](https://github.com/yogaardiansyah/rakitray/issues/new?template=module_request.yml) to verify that official portable archives (`.zip` / `.tar.gz` / `.tar.xz`) exist for target platforms.
2. **Verify Portable Operation:**
   A module **must not** require global OS registry modifications, system-wide Windows Services, or root-owned `/etc` configuration files. It must accept its configuration path, data directory, and port via CLI flags or local config templates.
3. **Define Exclusion Globs (`exclude_globs`):**
   Inspect the upstream archive and add glob patterns to strip debug symbols (`**/*.pdb`), test suites, and static libraries so users download and extract only what is needed at runtime.
4. **Provide Cryptographic Checksums:**
   Include official upstream `sha256` hashes for every supported OS and architecture.

---

## 6. Licensing & Attribution Sign-Off

By submitting a contribution to **Rakitray**, you agree that your contribution will be licensed under the project's **GNU Affero General Public License v3.0 only (`AGPL-3.0-only`)** alongside the mandatory attribution preservation terms in [`NOTICE`](./NOTICE). Do not remove or alter any copyright headers or attribution notices in [`LICENSE`](./LICENSE), [`NOTICE`](./NOTICE), UI components, or CLI output.

