# Rakitray v0.3.0 Engineering Contract (docs/V0\_3\_0\_CONTRACT.md)

This document defines the strict engineering contract, filesystem layout, 1:1 Rust-TypeScript data types, Tauri IPC command signatures, CLI surface, security rules, and step-by-step implementation checklist for milestone v0.3.0 (RakiStore Downloader, Lean Streaming Extractor, Service Blueprint Manifests & Quick-DB Provisioner).

## 1. Scope of Milestone v0.3.0

v0.3.0 turns Rakitray from a supervisor of pre-seeded binaries into a self-contained package manager and store:

- **RakiStore Catalog**: official manifest of downloadable runtimes per OS and architecture.
- **Async Chunk Downloader**: reqwest streaming, HTTP Range resume, exponential backoff, real-time progress over `tauri::ipc::Channel`.
- **SHA-256 Verifier**: zero-trust integrity check before anything touches `runtime/bin/`.
- **Lean Streaming Extractor**: strips bloatware at stream level (target: up to \~60% disk saving) with Anti-Zip-Slip sandboxing.
- **Service Blueprint (`rakit-manifest.toml`)**: per-version declarative lifecycle, replacing all bash wrapper scripts. Rust drivers read the blueprint instead of hardcoding engine behavior.
- **Per-instance config rendering** and **Quick-DB Provisioner** (create database, users, root password, `.env` snippet) over local Unix socket / named pipe.
- **Pure Clean Core**: the 4 seed runtimes from v0.1.0 / v0.2.0 (`nginx_1`, `mariadb_1`, `php_82`, `php_83`) are removed from the repository. Installer stays under \~8 MB.
- **RakiStore UI** (`RakiStorePanel.tsx`), **Empty Rack Onboarding** with 1-click stack templates, and CLI `rakit store | install | remove`.

Out of scope (deferred to v1.1+): Python/Go runtime drivers, auto-reverse-proxy for app runtimes, self-update of Rakitray itself, third-party catalog mirrors with signature verification.

## 2. Directory Layout Contract

All paths remain inside the self-contained `runtime/` root. Changes versus v0.1.0 are marked **(new)** or **(changed)**.

```text
runtime/
├── bin/                                   # (git-ignored) real binaries installed by RakiStore
│   └── <category>/<engine>/<version>/
│       ├── rakit-manifest.toml            # (new) Service Blueprint, written on install
│       └── ...engine files (bin/, share/, ext/, conf/)
├── store/                                 # (new) RakiStore working area
│   ├── catalog.json                       # cached catalog (embedded default is the fallback)
│   ├── installed.json                     # (new) install receipts: version, sha256, installedAt, strippedBytes
│   ├── downloads/                         # *.part (resumable) and verified archives
│   └── staging/                           # extraction target, atomically renamed on success
├── config/                                # (changed) generated per instance
│   ├── mariadb/mariadb_1.cnf
│   ├── postgres/postgres_1/postgresql.conf
│   ├── redis/redis_1.conf
│   ├── php/<version>-generated.ini
│   └── nginx/{nginx.conf,sites-enabled/}
├── data/<instance_id>/                    # (changed) per-instance persistent data
├── logs/<instance_id>.log
├── tmp/<instance_id>.{sock,pid}
└── www/
```

Git contract: `runtime/bin/`, `runtime/store/`, `runtime/data/`, `runtime/logs/`, `runtime/tmp/` and `runtime/config/` are in `.gitignore`. Only `runtime/www/default/` may be committed.

Instance ID format: `<engine>_<n>` (e.g. `mariadb_1`, `redis_1`). PHP instances keep `php_<major><minor>` style IDs for compatibility with v0.2.0 profiles.

## 3. Rust & TypeScript 1:1 Data Contracts

All models use `serde` with `rename_all = "camelCase"` unless stated. Enums use `snake_case` (or `kebab-case` for `Platform`). v0.1.0 models (`ServiceCategory`, `ServiceStatus`, `ServiceInstance`, `DiscoveredRuntime`, `IpcStreamEvent`) are unchanged.

### 3.1 Catalog

Rust (`src-tauri/src/models/store.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform { LinuxX64, LinuxArm64, WindowsX64, MacosX64, MacosArm64 }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat { TarGz, TarXz, Zip }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,                  // "mariadb@11.4.3"
    pub engine: String,
    pub category: ServiceCategory,
    pub version: String,
    pub platform: Platform,
    pub download_url: String,        // https only
    pub sha256: String,              // lowercase hex, 64 chars
    pub archive_format: ArchiveFormat,
    pub estimated_size_mb: u32,
    pub strip_components: u8,        // leading archive dirs to drop
    pub strip_profile: String,       // key into the strip rule table (section 6.3)
    pub blueprint_id: String,        // embedded blueprint template to write as rakit-manifest.toml
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub entry: CatalogEntry,
    pub installed: bool,
    pub install_path: Option<String>,
}
```

TypeScript (`src/types/store.ts`)

```ts
export type Platform = "linux-x64" | "linux-arm64" | "windows-x64" | "macos-x64" | "macos-arm64";
export type ArchiveFormat = "tar_gz" | "tar_xz" | "zip";

export interface CatalogEntry {
  id: string; engine: string; category: ServiceCategory; version: string;
  platform: Platform; downloadUrl: string; sha256: string;
  archiveFormat: ArchiveFormat; estimatedSizeMb: number;
  stripComponents: number; stripProfile: string; blueprintId: string;
}
export interface CatalogItem { entry: CatalogEntry; installed: boolean; installPath: string | null; }
```

### 3.2 Install Progress Streaming

Install progress uses its own channel (`Channel<StoreEvent>`), separate from the supervisor channel, so downloads never starve the 60Hz log/telemetry stream. Progress is throttled to \~16 ms per install.

Rust

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallState {
    Queued, Downloading, VerifyingSha256, ExtractingLean,
    Registering, Installed, Failed, Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub install_id: String,
    pub catalog_id: String,
    pub state: InstallState,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_secs: Option<u64>,
    pub stripped_bytes: u64,         // running total of bytes skipped by the lean extractor
    pub message: Option<String>,     // e.g. "Stripped 142 MB of bloatware (mysql-test, .pdb)"
    pub error: Option<StoreErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreErrorInfo { pub code: String, pub detail: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum StoreEvent {
    Progress(InstallProgress),
    InstalledRuntime(DiscoveredRuntime),   // emitted after scan_installed_runtimes re-runs
}
```

TypeScript

```ts
export type InstallState = "queued" | "downloading" | "verifying_sha256" | "extracting_lean"
  | "registering" | "installed" | "failed" | "cancelled";

export interface InstallProgress {
  installId: string; catalogId: string; state: InstallState;
  bytesDownloaded: number; totalBytes: number; speedBps: number; etaSecs: number | null;
  strippedBytes: number; message: string | null;
  error: { code: string; detail: string } | null;
}
export type StoreEvent =
  | { type: "progress"; payload: InstallProgress }
  | { type: "installedRuntime"; payload: DiscoveredRuntime };
```

### 3.3 Service Blueprint (`rakit-manifest.toml`)

TOML keys are `snake_case` (not camelCase). Rust parses with `toml` + `serde`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBlueprint {
    pub engine: BlueprintEngine,
    pub binaries: BlueprintBinaries,
    pub lifecycle: BlueprintLifecycle,
    pub config: Option<BlueprintConfig>,
    pub hooks: Option<BlueprintHooks>,
}

pub struct BlueprintEngine { pub name: String, pub category: ServiceCategory, pub version: String, pub default_port: Option<u16> }
pub struct BlueprintBinaries { pub daemon: String, pub client: Option<String>, pub installer: Option<String> }  // paths relative to bin_dir
pub struct BlueprintLifecycle {
    pub bootstrap_check_dir: Option<String>,
    pub bootstrap_cmd: Option<Vec<String>>,   // runs once if bootstrap_check_dir is missing
    pub start_args: Vec<String>,
    pub ready_check: ReadyCheck,              // tcp port | unix socket | log regex, with timeout_secs
    pub stop_signal: Option<String>,          // default SIGTERM, then SIGKILL after grace period
}
pub struct BlueprintConfig { pub template: String, pub output: String }   // template file shipped in the blueprint
pub struct BlueprintHooks { pub post_start: Option<String> }              // e.g. "provision_mariadb"
```

Template variables (the only ones allowed): `{{instance_id}}`, `{{version}}`, `{{bin_dir}}`, `{{data_dir}}`, `{{config_file}}`, `{{port}}`, `{{socket_path}}`, `{{pid_path}}`, `{{log_file}}`, `{{bind_address}}`. Unknown variables fail blueprint validation. Rendered argv entries are passed as separate arguments; they are never concatenated into a shell string.

Reference blueprint:

```toml
[engine]
name = "mariadb"
category = "database"
version = "11.4.3"
default_port = 3306

[binaries]
daemon = "bin/mariadbd"
client = "bin/mariadb"
installer = "scripts/mariadb-install-db"

[lifecycle]
bootstrap_check_dir = "{{data_dir}}/mysql"
bootstrap_cmd = [
  "{{bin_dir}}/scripts/mariadb-install-db",
  "--no-defaults",
  "--datadir={{data_dir}}",
  "--basedir={{bin_dir}}",
  "--auth-root-authentication-method=normal",
]
start_args = [
  "--defaults-file={{config_file}}",
  "--basedir={{bin_dir}}",
  "--datadir={{data_dir}}",
  "--port={{port}}",
  "--socket={{socket_path}}",
  "--pid-file={{pid_path}}",
  "--console",
]
ready_check = { kind = "unix_socket", timeout_secs = 30 }

[hooks]
post_start = "provision_mariadb"
```

Version differences live in the manifest, not in Rust. MySQL 8.x uses `bootstrap_cmd = ["{{bin_dir}}/bin/mysqld", "--initialize-insecure", "--datadir={{data_dir}}"]`. PostgreSQL uses `initdb -D {{data_dir}}` and `postgres -D {{data_dir}} -p {{port}}`.

Guarantee: the Supervisor spawns `binaries.daemon` directly (no `sh -c`), so the tracked PID is the real engine PID and v0.1.0 anti-zombie guarantees (`PR_SET_PDEATHSIG`, PGID, Windows Job Objects) and RAM/CPU telemetry remain accurate.

### 3.4 Config Patch

Replaces the v0.1.0 placeholder `ConfigPatch`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPatch {
    pub port: Option<u16>,
    pub bind_address: Option<String>,
    pub profile: Option<String>,                        // dev | prod (PHP matrix from v0.2.0)
    pub tunables: Option<std::collections::BTreeMap<String, String>>, // e.g. innodb_buffer_pool_size = "256M"
    pub restart: bool,                                  // true = "Save & Reload"
}
```

```ts
export interface ConfigPatch {
  port?: number; bindAddress?: string; profile?: string;
  tunables?: Record<string, string>; restart: boolean;
}
```

Behavior: validate port (free, not reserved) -> persist instance state -> re-render config from template into `runtime/config/...` -> if `restart`, stop then start with the new config. Files inside `runtime/bin/` are never edited.

### 3.5 Quick-DB Provisioner

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCreateRequest { pub name: String, pub charset: Option<String>, pub collation: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbUserRequest { pub username: String, pub password: String, pub grant_database: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbOverview { pub databases: Vec<String>, pub users: Vec<String>, pub root_password_set: bool }
```

```ts
export interface DbCreateRequest { name: string; charset?: string; collation?: string; }
export interface DbUserRequest { username: string; password: string; grantDatabase?: string; }
export interface DbOverview { databases: string[]; users: string[]; rootPasswordSet: boolean; }
```

### 3.6 Stack Templates

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTemplate { pub id: String, pub name: String, pub description: String, pub members: Vec<String> } // catalog ids
```

```ts
export interface StackTemplate { id: string; name: string; description: string; members: string[]; }
```

Built-in templates: `laravel-modern` (nginx@1.26 + php@8.3 + mariadb@11.4 + redis@7.2), `legacy-php` (nginx@1.26 + php@8.2 + mariadb@11.4), `data-only` (mariadb@11.4 + redis@7.2).

## 4. Tauri Commands Contract

All v0.1.0 commands remain. `update_instance_config` now takes the real `ConfigPatch` from 3.4. New commands:

| Command | Rust Signature | Description |
| --- | --- | --- |
| `list_catalog` | `pub async fn list_catalog(refresh: bool, state: State<AppState>) -> Result<Vec<CatalogItem>, RakitrayError>` | Catalog filtered to the current Platform, flagged installed/available. `refresh` re-fetches remote catalog, falling back to cache/embedded. |
| `install_runtime` | `pub async fn install_runtime(catalog_id: String, on_event: Channel<StoreEvent>, state: State<AppState>) -> Result<DiscoveredRuntime, RakitrayError>` | Download, verify, lean-extract, register. Calls `scan_installed_runtimes` on success. |
| `cancel_install` | `pub async fn cancel_install(install_id: String, state: State<AppState>) -> Result<(), RakitrayError>` | Cancels in-flight install, keeps `.part` for resume, cleans staging. |
| `uninstall_runtime` | `pub async fn uninstall_runtime(catalog_id: String, state: State<AppState>) -> Result<(), RakitrayError>` | Refuses if any instance of it is running. Removes bin dir and receipt; data dirs are kept unless `purge` is confirmed in UI. |
| `list_stack_templates` | `pub fn list_stack_templates() -> Vec<StackTemplate>` | Quick-start templates. |
| `install_stack` | `pub async fn install_stack(template_id: String, on_event: Channel<StoreEvent>, state: State<AppState>) -> Result<Vec<DiscoveredRuntime>, RakitrayError>` | Installs members in parallel (max 3 concurrent), creates instances in the Rack. |
| `db_overview` | `pub async fn db_overview(instance_id: String, state: State<AppState>) -> Result<DbOverview, RakitrayError>` | Lists databases and users via local socket. |
| `db_create_database` | `pub async fn db_create_database(instance_id: String, req: DbCreateRequest, state: State<AppState>) -> Result<(), RakitrayError>` | Idempotent `CREATE DATABASE IF NOT EXISTS` (or Postgres equivalent). |
| `db_create_user` | `pub async fn db_create_user(instance_id: String, req: DbUserRequest, state: State<AppState>) -> Result<(), RakitrayError>` | Creates user and grants privileges on `grant_database`. |
| `db_set_root_password` | `pub async fn db_set_root_password(instance_id: String, password: String, state: State<AppState>) -> Result<(), RakitrayError>` | Default dev mode is an empty root password. |
| `db_env_snippet` | `pub async fn db_env_snippet(instance_id: String, database: String, state: State<AppState>) -> Result<String, RakitrayError>` | Returns a ready-to-paste `.env` block. |
| `cache_flush` | `pub async fn cache_flush(instance_id: String, state: State<AppState>) -> Result<(), RakitrayError>` | Redis/Valkey FLUSHALL. |

Identifiers (database, username) are validated against `^[A-Za-z0-9_]{1,64}$` and quoted by the driver; passwords are passed through parameterized statements or escaped by the driver, never string-concatenated from raw input.

## 5. CLI Contract (`rakit`)

```bash
rakit store list [--category <c>] [--json]   # catalog with installed/available state
rakit install <engine>@<version> [...]       # multiple args allowed, e.g. nginx@1.26.2 php@8.3.12
rakit install --stack laravel-modern
rakit remove <engine>@<version> [--purge]
```

- Terminal progress bar shows state, `45.2 MB / 120.0 MB`, speed, ETA and the stripped-bloatware summary.
- Exit codes: `0` success, `1` generic failure, `2` unknown catalog id, `3` checksum mismatch, `4` network failure after retries, `5` runtime in use.
- CLI and GUI share the same `store` module; no logic lives only in the CLI.

## 6. Security & Extraction Contracts

### 6.1 Downloader (`src-tauri/src/store/downloader.rs`)

- HTTPS only; non-HTTPS `download_url` is rejected at catalog validation.
- Streams to `runtime/store/downloads/<id>.part`. On interruption, resumes with `Range: bytes=<len>-`. If the server ignores Range (200 instead of 206) the file restarts.
- Retry: exponential backoff 1s, 2s, 4s, 8s, 16s (max 5 attempts), then `Failed` with code `network`.
- Progress emitted at most every 16 ms; speed uses a 2-second moving average.

### 6.2 SHA-256 Verifier (`sha2` crate)

- Hash is computed over the complete archive (re-reading the `.part` from byte 0 when resumed) before extraction starts.
- Mismatch: delete the archive, nothing is written to `runtime/bin/`, state `Failed` with code `checksum_mismatch` (CLI exit 3).
- Comparison is constant-time on lowercase hex.

### 6.3 Lean Streaming Extractor (`src-tauri/src/store/extractor.rs`)

Entries are filtered in memory before any bytes reach disk. Skipped entries add to `strippedBytes`.

| Rule group | Patterns |
| --- | --- |
| Debug & static libs | `**/*.pdb`, `**/*.lib`, `**/*.exp`, `**/*.a` |
| DB test/bench suites | `mysql-test/**`, `sql-bench/**` |
| Offline docs | `doc/**`, `docs/**`, `manual/**`, `man/**` |

Rules are keyed by `strip_profile` so an engine can opt out (e.g. a runtime whose `.lib` files are required at runtime). Strip patterns are applied after `strip_components`.

### 6.4 Anti-Zip-Slip Sandbox

For every entry (zip and tar):

1. Reject absolute paths, Windows drive/UNC prefixes, and any `..` component.
2. Join onto the staging root and verify the normalized path still starts with the staging root.
3. Reject symlinks and hardlinks whose target resolves outside the staging root.
4. Reject device nodes and entries with setuid/setgid bits (permissions are masked).
5. On violation: abort the whole extraction, delete staging, state `Failed` with code `path_traversal`.

Extraction targets `runtime/store/staging/<id>/` and is atomically renamed to `runtime/bin/<category>/<engine>/<version>/` only after the full archive succeeds and the blueprint validates. A crash never leaves a half-installed runtime in `bin/`.

### 6.5 Catalog Trust

The catalog embedded in the app binary is trusted. A remote refresh is accepted only over HTTPS and only if it passes schema validation; SHA-256 values always come from the catalog. (Signed catalogs are planned for a later milestone.)

## 7. Quick-DB Provisioner Behavior

- Runs from the blueprint hook `post_start = "provision_mariadb"` (similar hooks for MySQL and Postgres) when the instance reaches `Running`.
- Connects only via the instance Unix socket (`runtime/tmp/<instance_id>.sock`) or named pipe on Windows; never over an exposed TCP port for admin tasks.
- Dev default: `root@127.0.0.1` with an empty password so a stock Laravel `.env` works with no edits. Setting a root password flips `rootPasswordSet` and is reflected in the `.env` snippet.
- Default bind address is `127.0.0.1`. Binding to `0.0.0.0` requires an explicit confirmation in the UI.

`.env` snippet format (port and database reflect the live instance):

```env
DB_CONNECTION=mysql
DB_HOST=127.0.0.1
DB_PORT=3307
DB_DATABASE=hello_laravel
DB_USERNAME=root
DB_PASSWORD=
```

## 8. Empty Rack Onboarding

When `scan_installed_runtimes` returns 0 runtimes, the Service Rack renders Quick-Start cards from `list_stack_templates`: **Laravel Modern Stack**, **Legacy PHP Stack**, and **Custom / Pick Your Own** (opens the RakiStore tab). Installing a stack runs member installs concurrently (max 3), then creates one instance per runtime so the Rack is ready to Start without an app restart.

## 9. Migration from v0.2.0

1. Delete seed runtimes (`nginx_1`, `mariadb_1`, `php_82`, `php_83`) and seed `.so` extension files from the repo; add the git-ignore rules from section 2.
2. Replace bash wrapper launching with blueprint-driven spawning. Drivers keep engine-specific logic only for config rendering and hooks.
3. Move the v0.2.0 PHP Matrix output to `runtime/config/php/<version>-generated.ini` driven by the PHP blueprint; extension toggles keep working against real `ext/` contents.
4. Existing users with legacy `runtime/data/mariadb/` are migrated to `runtime/data/mariadb_1/` on first launch (move, never copy-delete).
5. Bump `scan_installed_runtimes` to treat a directory without `rakit-manifest.toml` as `isValid = false`.

## 10. v0.3.0 Implementation Checklist

**Phase A: Models & Contracts**

- [ ] Rust models in `src-tauri/src/models/store.rs` (Platform, ArchiveFormat, CatalogEntry, CatalogItem, InstallState, InstallProgress, StoreEvent, StackTemplate, Db\* types, ConfigPatch).
- [ ] Blueprint structs and TOML parser with variable validation in `src-tauri/src/blueprint/`.
- [ ] Mirrored TypeScript types in `src/types/store.ts`.

**Phase B: Catalog**

- [ ] Embedded `catalog.json` for linux-x64, windows-x64, macos-arm64 (Nginx 1.26/1.27, Caddy 2.8, PHP 8.1-8.4, Bun/Node, MariaDB 11.4, PostgreSQL, Redis 7.2, Mailpit 1.21, Composer).
- [ ] Platform detection and filtering; optional remote refresh with cache fallback.

**Phase C: Downloader, Verifier, Extractor**

- [ ] `store/downloader.rs`: streaming, Range resume, backoff, throttled progress, cancel.
- [ ] SHA-256 verification with delete-on-mismatch.
- [ ] `store/extractor.rs`: zip + tar.gz/xz, strip rules, Anti-Zip-Slip, staging + atomic rename.
- [ ] Unit tests: traversal payloads (`../../etc/passwd`, absolute paths, symlink escape), checksum mismatch, resume after cut connection, strip accounting.

**Phase D: Blueprint-Driven Drivers & Config**

- [ ] Refactor drivers to read `rakit-manifest.toml` (bootstrap, start args, ready check, stop signal).
- [ ] Per-instance config renderer and real `update_instance_config` (Save & Reload).
- [ ] Blueprints for nginx, caddy, php-cgi, mariadb, postgres, redis, mailpit.

**Phase E: Quick-DB Provisioner**

- [ ] Socket/named-pipe admin client; commands `db_*` and `cache_flush`.
- [ ] Input validation and quoting; `.env` snippet generator.

**Phase F: rakit CLI**

- [ ] `rakit store list`, `rakit install` (multi-arg and `--stack`), `rakit remove`, exit codes from section 5.

**Phase G: SolidJS RakiStore UI**

- [ ] `RakiStorePanel.tsx`: catalog by category, Installed/Available badges, Install/Uninstall actions.
- [ ] 60Hz-smooth progress bar with speed, ETA and stripped-bloatware message.
- [ ] Empty Rack Onboarding cards and stack install.
- [ ] MariaDB card: Create Database, Create App User, Root Password, Copy `.env` DB Config.
- [ ] Auto-call `scan_installed_runtimes` after install so new runtimes appear without restart.

## 11. Acceptance Criteria

- Fresh clone has zero binaries in `runtime/bin/`; app installer is under \~8 MB.
- `rakit install mariadb@11.4.3` on a clean machine ends with MariaDB visible in the Rack and startable, with installed size at least \~50% smaller than the raw archive contents where test/doc/debug files exist.
- A tampered archive (one byte changed) is rejected with exit code 3 and leaves `runtime/bin/` untouched.
- A crafted archive containing `../../evil` is rejected with `path_traversal` and writes nothing outside staging.
- Killing the network mid-download and retrying resumes from the `.part` file.
- `laravel-modern` stack installs, starts, and serves `runtime/www/default/index.php` through Nginx + PHP 8.3 with `php artisan migrate` working against the provisioned MariaDB using the copied `.env` snippet.
- Stopping Rakitray (or killing it) leaves no orphaned engine processes (v0.1.0 anti-zombie guarantee still holds).
