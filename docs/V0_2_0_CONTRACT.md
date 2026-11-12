# Rakitray v0.2.0: Visual PHP Matrix & Dual-Profile Engine

> Planning document. Analysis basis: `README.md` and `docs/ARCHITECTURE.md` (branch `main`).
> `AGENTS.md`, `CHANGELOG.md`, and the contents of `src-tauri/` were **not readable** during analysis, so every path in this plan must be verified via task T0 before writing code.

## 1. Goal

Laravel developers should never have to open `php.ini` by hand again. PHP extensions, performance profiles (dev vs prod), and cross-version sync are controlled from the UI, and the PHP worker is restarted automatically.

### Five pillars

1. **Dynamic PHP Extension Scanner** (`src-tauri/src/discovery/php_ext.rs`): scans each installed PHP version for extensions and their state (enabled/disabled), grouped by category.
2. **Dual-Profile Engine** (`src-tauri/src/drivers/php.rs`): generates `<version>-generated.ini` for a `dev` or `prod` profile.
3. **1-Click Laravel Preset**: enables the Laravel extensions and restarts the worker.
4. **Profile Clone** between PHP versions.
5. **"PHP Matrix" UI** (`PhpMatrixPanel.tsx`) in the sidebar, below Sites & Routing.

### Data contract (1:1 Rust ↔ TypeScript)

Files: `src-tauri/src/models/php_matrix.rs` and `src/types/phpMatrix.ts`.

| Tauri command | Purpose |
|---|---|
| `get_php_matrix(instance_id)` | Extension list, state, profile, and limits for that PHP version |
| `toggle_php_extension(instance_id, ext_name, enabled)` | Enable/disable an extension, then restart running instances |
| `apply_php_profile(instance_id, profile, limits)` | Switch dev/prod with limits, regenerate the ini, restart the worker |
| `apply_laravel_preset(instance_id)` | Enable the standard Laravel extensions (+ Redis if available), restart the worker |
| `clone_php_matrix(source_instance_id, target_instance_id)` | Copy enabled extensions, profile, and limits across versions |

## 2. Verification against the repo

Consistent with the plan:

- ARCHITECTURE defines the runtime hierarchy as `runtime/bin/<category>/<engine>/<version>/` (example `runtime/php-cgi/8.3.12/`). The path `runtime/bin/runtime/php-cgi/<version>/ext/` is consistent.
- The frontend is **forbidden** from parsing or templating `php.ini` and from polling. All logic lives in Rust; the UI only uses `invoke()` and `Channel`.
- The UI follows Atomic Design. `PhpMatrixPanel` goes in `src/components/organisms/` and must use SolidJS primitives, not React.
- Existing inconsistency: README mentions `.dev.toml`, ARCHITECTURE mentions `.rakitray.toml`. v0.2.0 must not make this worse.

## 3. Critical findings and design decisions

### a. `.so` extensions may not exist on Linux/macOS
Official Windows PHP builds ship `ext/php_*.dll`. For Linux/macOS, the source of Rakitray's PHP binaries is unknown. If they are static builds (for example static-php-cli), most extensions are *compiled in* and cannot be toggled through `.ini`. This is an unverified assumption about Rakitray's registry.
**Decision:** a hybrid scanner: scan the `ext/` folder plus run `php -m` to detect built-in extensions (shown read-only).

### b. The "Laravel Core" list differs from the official one
The Laravel 12 docs (laravel.com/docs/12.x/deployment) require PHP >= 8.2 and these extensions: Ctype, cURL, DOM, Fileinfo, Filter, Hash, Mbstring, OpenSSL, PCRE, PDO, Session, Tokenizer, XML.

- `bcmath` and `zip` are not in the official list (recommended, not required).
- `redis` is a PECL extension and is not bundled with PHP. On Windows its DLL is downloaded separately.
- `ctype`, `tokenizer`, `filter`, `hash`, `session`, and `pcre` are usually built in and have no DLL.

**Decision:** the preset means "ensure available", with three tiers:

| Tier | Extensions |
|---|---|
| Required | ctype, curl, dom, fileinfo, filter, hash, mbstring, openssl, pcre, pdo, session, tokenizer, xml |
| Recommended | pdo_mysql, zip, bcmath, intl, gd, exif |
| Optional | redis |

### c. Outdated version labels
Laravel 12 supports PHP 8.2 to 8.4. Laravel 13 (released March 2026) requires PHP 8.3+. The label "Laravel 11/12" becomes "Laravel 12/13". Do not hardcode 8.2/8.3 in the version selector; read it from the installed versions.

### d. Xdebug and OPcache
- Port 9003 is set via `xdebug.client_port` (the Xdebug 3 default), not as part of `xdebug.mode`.
- Prod mode **does not load** Xdebug at all. `xdebug.mode=off` still leaves a small overhead.
- On Windows, Xdebug is a separate DLL that must match the PHP version, TS/NTS, and architecture.
- OPcache is loaded through `zend_extension=`, not `extension=`. To the author's knowledge, PHP 8.5 makes OPcache always built in. **Needs verification** before handling 8.5.

### e. Extension load order
Example: `exif` must load after `mbstring` (especially on Windows). The renderer uses topological order, not alphabetical.

### f. Restart storm
`php-cgi` has no reload, so "reload" means stop then start, and in-flight requests are dropped. Restarting the worker on every checkbox click would feel bad.
**Decision:**
- Changes are held as *pending* and debounced (about 500 ms), or applied with an Apply button.
- Validate the new `.ini` with `php -n -c <file> -m` before restarting.
- Write atomically (temp file then rename), keep a `.bak`, and roll back automatically if validation or startup fails.

### g. Single source of truth
Reading the profile back from `.ini` is fragile.
**Decision:**
- Structured state in `runtime/config/php/<version>.matrix.toml` (source of truth).
- `<version>-generated.ini` is render output only.
- Manual user overrides live in `<version>-user.ini` via `PHP_INI_SCAN_DIR`, so they are not overwritten on regenerate.
- Kept separate from `.rakitray.toml` to avoid conflicts.

### h. Input security
`ext_name` from the frontend must never be written to `.ini` directly (risk of path traversal and ini injection such as `extension=../../x.dll` or newlines in limit values).
**Decision:**
- `ext_name` is validated against the scan results (allowlist).
- Limits are validated with the regex `^(-1|\d+[KMG]?)$` and no control characters.

### i. `instance_id` vs `<version>`
Config is per version, but commands take an `instance_id`.
**Decision:** the instance resolves to its version; a change applies to all instances of that version and all are restarted. `clone_php_matrix` rejects a clone when source and target versions are the same.

## 4. Work order

```
T0 → T1 → T2 ─┐
        └→ T3 ─┼→ T4 → T5 → T6 → T7, T8 → T9 → T10 → T11 → T12
```

T2 and T3 can run in parallel. T7 and T8 can too.

## 5. Master prompt (paste at the start of every task)

```
You are working on Rakitray (Tauri v2 + Rust/Tokio + SolidJS + Bun). Read AGENTS.md
and docs/ARCHITECTURE.md first. Hard rules:
- All file reads/writes, process spawning, and ini/toml rendering happen ONLY in Rust (src-tauri/).
  Forbidden in src/: plugin-fs, plugin-shell, php.ini parsing, setInterval+invoke polling.
- Telemetry/events are pushed from Rust via tauri::ipc::Channel.
- UI: Atomic Design (atoms/molecules/organisms/templates), SolidJS primitives only,
  no heavy UI libraries, no React.
- The Rust <-> TypeScript contract must be 1:1 (serde rename_all = "camelCase").
- Errors use RakitrayError; no unwrap()/expect() on production paths.
- Done only when these pass: cargo fmt --check, cargo clippy -D warnings, cargo test,
  bunx biome check ., bun run tsc --noEmit.
- Follow the license/header rules in AGENTS.md. Do not touch files outside the task scope.
- If an assumption in the prompt contradicts the real code, STOP and report; do not guess.
```

## 6. Task prompts

### T0: Recon (read-only, no code changes)

```
Goal: verify the v0.2.0 "PHP Matrix" assumptions before writing code. Do not modify files.
Inspect and report with file paths + line numbers:
1. Where do the PHP binaries come from (registry/manifest)? What does the build look like per OS:
   is there an ext/ folder with php_*.dll / *.so, or are extensions compiled in/static?
2. Contents of src-tauri/src/drivers/php.rs: is there already a php.ini generator, where is
   the file written, are php-fpm and php-cgi modes distinguished?
3. The ServiceInstance model: which field maps instance_id → PHP version?
   Can there be more than one instance per version?
4. How does the supervisor stop/start/restart one instance, and how are log lines sent
   from Rust to DockedLogTerminal?
5. Structure of src/components/organisms/SidebarNav and src/store/supervisorStore.ts:
   how to add a new menu item and extend the store.
6. Version/CHANGELOG conventions, and whether src-tauri/src/discovery and models folders exist.
Output: a report + a list of differences between the plan (see PLAN-v0.2.0.en.md) and the actual code.
```

### T1: Rust ↔ TypeScript data contract

```
Create src-tauri/src/models/php_matrix.rs and src/types/phpMatrix.ts (1:1, camelCase).
Types: PhpProfileMode {Dev, Prod}; ExtensionKind {Extension, ZendExtension};
ExtensionSource {SharedLib, BuiltIn}; ExtensionCategory {LaravelCore, DatabaseCache,
PerformanceDebug, MediaFormat, Other}; PhpExtension {name, kind, source, category,
enabled, toggleable, requires: Vec<String>};
PhpIniLimits {memory_limit, upload_max_filesize, post_max_size, max_execution_time};
PhpMatrix {instance_id, version, profile, limits, extensions, warnings: Vec<String>}.
Also create PhpMatrixEvent (Channel) for apply status: Applying, Reloaded, RolledBack{reason}.
Add serde round-trip unit tests (JSON snapshot). Do not implement logic.
Acceptance: serde tests pass; TS fields are identical to the JSON produced by Rust.
```

### T2: Dual-profile engine (pure function)

```
In src-tauri/src/drivers/php.rs (or php_profile.rs if the file gets too large) create a pure
function render_ini(version, profile, limits, enabled_exts, ext_dir) -> String.
Dev: display_errors=On, display_startup_errors=On, error_reporting=E_ALL,
memory_limit=512M, opcache.revalidate_freq=0, xdebug.mode=debug,develop,
xdebug.client_port=9003.
Prod: display_errors=Off, log_errors=On, memory_limit=1024M, opcache.enable=1,
opcache.validate_timestamps=0, Xdebug is NOT loaded (not merely mode=off).
User limits override profile defaults. Rules: set extension_dir explicitly;
zend_extension= for opcache/xdebug, extension= for the rest; topological order
(mbstring before exif, etc.); opcache.enable_cli controlled separately.
Output must be deterministic (stable ordering) so it can be snapshot-tested.
Validate limits with the regex ^(-1|\d+[KMG]?)$; reject control characters/newlines;
warn if upload_max_filesize > post_max_size.
Tests: dev and prod snapshots, ordering, injection rejected.
```

### T3: Extension scanner

```
Create src-tauri/src/discovery/php_ext.rs.
scan(php_root) -> Vec<PhpExtension>:
1. Enumerate <php_root>/ext: Windows php_<name>.dll, Linux/macOS <name>.so.
   Normalize names (strip the "php_" prefix).
2. Run "<php-bin> -n -m" (5s timeout) to get compiled-in extensions;
   those without a shared file → source=BuiltIn, toggleable=false.
3. Categorize: LaravelCore (pdo_mysql, mbstring, openssl, curl, fileinfo,
   bcmath, tokenizer, xml/dom, zip, ctype), DatabaseCache (redis, sqlite3, pgsql,
   memcached), PerformanceDebug (opcache, xdebug, pcov), MediaFormat (gd, intl,
   exif, imagick). Everything else → Other.
4. Mark zend_extension for opcache, xdebug, pcov (static table).
5. Enabled state: parse generated.ini / matrix.toml in Rust (not TS).
Use tokio::fs for I/O; do not panic if the ext folder is missing (return empty + warning).
Tests: fake directory fixtures for Windows and Linux layouts.
```

### T4: State store, atomic writes, rollback

```
Build the config persistence module: runtime/config/php/<version>.matrix.toml
(profile, limits, enabled_exts) as the source of truth, and <version>-generated.ini
as render output. Support <version>-user.ini via PHP_INI_SCAN_DIR (never overwrite it).
write_matrix(): write to a temp file → fsync → rename; keep <version>-generated.ini.bak;
validate with "<php-bin> -n -c <tmp.ini> -m" (exit code + stderr parsed for
PHP "Unable to load dynamic library" warnings/errors). On failure the old file
stays and a RakitrayError with the reason is returned. Add a per-version lock (tokio Mutex)
so two applies cannot run at once.
Tests: successful write, failed validation (fake extension), a crash mid-write does not corrupt the old file.
```

### T5: Apply + restart orchestration

```
Build apply_matrix(instance_id, change) in Rust: resolve instance → version → all
instances of that version; write (T4); if any instance is running, restart via the supervisor
(5s grace stop → start) only AFTER validation passes. If start fails → roll back
to .bak and restart again, then send PhpMatrixEvent::RolledBack.
Debounce: queue changes per version and merge them within a ~500ms window into one restart.
Send a log line to the bottom terminal: "[PHP Matrix] Applied <what> to php_<version> & reloaded worker".
If php-fpm is available, graceful reload (SIGUSR2) is allowed; for php-cgi: restart.
Tests: operation order with a mock supervisor; rollback path.
```

### T6: The five Tauri commands

```
In src-tauri/src/commands/php_matrix.rs implement and register in
tauri::Builder (generate_handler!): get_php_matrix(instance_id),
toggle_php_extension(instance_id, ext_name, enabled), apply_php_profile(instance_id,
profile, limits), apply_laravel_preset(instance_id), clone_php_matrix(source, target).
All return Result<PhpMatrix, RakitrayError>.
Rules: ext_name must exist in the scan results (allowlist) and be toggleable; built-in
extensions are rejected with a clear message; extension dependencies (requires) are checked.
Reject clone if the versions are equal or either instance is not found.
Add a Channel-based command for PhpMatrixEvent (push, not polling).
Tests: integration test per command using fixtures; hostile ext_name cases
("../x", "a\nb", unknown names).
```

### T7: Laravel preset

```
Implement apply_laravel_preset with 3 tiers: required (ctype, curl, dom,
fileinfo, filter, hash, mbstring, openssl, pcre, pdo, session, tokenizer, xml),
recommended (pdo_mysql, zip, bcmath, intl, gd, exif), optional (redis).
Behavior: enable those that have a shared file; built-in ones are only verified; missing
ones go into warnings (e.g. "redis not found: place php_redis.dll in ext/"),
do NOT fail entirely. The preset does not change the dev/prod profile and does not disable
extensions the user chose. Check the PHP version: warn if < 8.2 (Laravel 12) or < 8.3 (Laravel 13).
Result: PhpMatrix + a diff list (enabled / already present / missing).
Tests: full matrix, matrix without redis, PHP 8.1 matrix.
```

### T8: Profile clone between versions

```
Implement clone_php_matrix: copy profile, limits, and the enabled extension list
from source to target ONLY for extensions available on the target (intersection of scan results).
Extensions missing on the target → warnings, not errors. Do not copy extension_dir
or absolute paths. Persist through T4/T5 (validation + rollback + restart of the target).
Tests: clone 8.2→8.3 with an extension missing on the target; clone to the same version is rejected.
```

### T9: Frontend store and invoke wrappers

```
In src/ add: src/lib/phpMatrixApi.ts (typed invoke wrappers),
and extend src/store/supervisorStore.ts or create phpMatrixStore.ts (createStore + produce).
State: selected version, matrix per version, pending changes, apply status, warnings.
The matrix is fetched on demand via invoke (once, not polling) and updated
from PhpMatrixEvent via Channel. Optimistic UI with rollback on a RolledBack event.
Forbidden: ini/toml parsing, plugin-fs, setInterval. Test with a mock invoke if
frontend test infrastructure exists; otherwise type-check is enough.
```

### T10: PhpMatrixPanel UI

```
Build the UI following Atomic Design:
- atoms: ExtensionCheckbox (built-in/missing state, disabled + tooltip), ProfileBadge.
- molecules: ProfileToggle (DEV/PROD), LimitField (memory_limit, upload_max_filesize,
  max_execution_time with format validation), PendingBar (Apply/Discard).
- organism: src/components/organisms/PhpMatrixPanel.tsx (SolidJS): version selector
  from installed versions, extension grid per category, "1-Click Laravel Preset" button
  and "Clone to…" (dropdown of other versions, confirmation dialog with the diff list).
- Add a "PHP Matrix" item to SidebarNav, below "Sites & Routing", and its route in WorkstationShell.
Show warnings and apply status. Accessible (keyboard, aria). No new UI libraries.
Acceptance: biome + tsc pass; toggle/preset/clone work end-to-end in `bun run tauri dev`.
```

### T11: Testing and CI

```
Make sure CI (.github workflow ci.yml) runs cargo fmt/clippy/test and biome/tsc.
Add a fake PHP fixture (ext dir + a stub binary script that mimics "-m") so tests
do not need a real PHP. Add a manual test checklist in docs: Windows (DLL) and Linux
(static vs shared build). Report coverage of the critical areas: ini renderer, allowlist, rollback.
```

### T12: Documentation and release

```
Update docs/ARCHITECTURE.md (discovery/php_ext module, models/php_matrix, apply+rollback flow,
matrix.toml file), README (PHP Matrix feature), and CHANGELOG.md for v0.2.0 following the
existing format. Bump the version in package.json, Cargo.toml, tauri.conf.json consistently.
```

## 7. Open decisions (decide before T1)

- **PHP binary source on Linux/macOS** (result of T0): determines whether extension toggling works there or only on Windows.
- **Auto-apply vs an Apply button:** recommendation is pending + debounce, not a restart per click.
- **Scope of the "PHP Matrix" menu:** recommendation is per version, with the instance resolved in the backend.

## 8. Definition of Done for v0.2.0

- The five commands work and are tested, including hostile-input cases.
- Extension toggle, profile switch, preset, and clone work end-to-end in the UI.
- Config is never corrupted: atomic writes, validation, and rollback are proven by tests.
- Built-in/unavailable extensions are handled with clear warnings, not errors.
- CI is green (`cargo fmt`, `clippy -D warnings`, `cargo test`, `biome`, `tsc`).
- Documentation and CHANGELOG are updated.