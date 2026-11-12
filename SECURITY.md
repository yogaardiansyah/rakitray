# Security Policy

**Rakitray** is a local development environment launcher that downloads and extracts binary archives, orchestrates child OS processes (web servers, databases, language runtimes), binds local network ports, and optionally updates local `hosts` files. Because Rakitray operates close to the operating system, security is treated as a first-class engineering priority.

---

## Supported Versions

Only the latest release on the active `0.x` development branch receives security patches:

| Version | Supported |
| :--- | :--- |
| `0.1.x` (Current) | :white_check_mark: Supported |
| `< 0.1.0` | :x: Not Supported |

---

## Threat Model & In-Scope Vulnerability Classes

We actively monitor and accept security reports for the following attack vectors specific to Rakitray's architecture:

### 1. Command Injection & Process Spawning (`HIGH / CRITICAL`)
- Untrusted input from project `.dev.toml` files, environment variables, or service configuration templates being passed unsafely into shell execution.
- *Mitigation enforced in Rakitray:* Child processes must always be spawned directly via `std::process::Command` / `tokio::process::Command` with explicit argument vectors (never `sh -c` or `cmd.exe /C` string interpolation).

### 2. Archive Path Traversal / "Zip Slip" (`CRITICAL`)
- Maliciously crafted `.zip`, `.tar.gz`, or `.tar.xz` archives containing `../` segments, absolute paths, or symbolic/hard links attempting to write files outside the designated `bin/<category>/<module>/<version>/` sandbox directory.
- *Mitigation enforced in Rakitray:* All archive entry paths are canonicalized and validated against the target sandbox prefix prior to file creation; symlinks pointing outside the extraction root are rejected immediately.

### 3. Privilege Escalation via `hosts` File or Port Binding (`CRITICAL`)
- Improper validation when updating system `hosts` files (`/etc/hosts` or `C:\Windows\System32\drivers\etc\hosts`) that could allow arbitrary file modification or injection of non-loopback (`127.0.0.1` / `::1`) DNS mappings.
- *Mitigation enforced in Rakitray:* The `hosts` manager strictly validates domain names against RFC-compliant `.test` / `.local` / `.localhost` patterns and only writes entries within a fenced `# BEGIN RAKITRAY` / `# END RAKITRAY` block mapped exclusively to loopback addresses.

### 4. Binary Supply-Chain & Checksum Verification Bypass (`HIGH`)
- Downloading or extracting runtime modules over non-TLS connections or skipping cryptographic SHA-256 verification before execution.
- *Mitigation enforced in Rakitray:* Only `https://` URLs are permitted, and SHA-256 digests are verified in-stream before the extracted staging directory is committed.

### 5. Tauri v2 IPC & Capability Boundary Bypass (`HIGH`)
- Exposing unrestricted filesystem or shell capabilities to the WebView frontend or allowing untrusted local web pages to invoke Tauri IPC commands.

---

## Out of Scope

The following items are generally considered **out of scope** for Rakitray security advisories:
- Vulnerabilities inside upstream third-party binaries downloaded by the user (e.g., a CVE in upstream PHP, Nginx, or MariaDB itself)—please report those directly to the respective upstream vendor, and open a regular issue or PR in Rakitray to bump the catalog version.
- Intentional user configuration of `.dev.toml` to bind a development service to `0.0.0.0` (LAN) instead of `127.0.0.1`.
- Attacks requiring prior physical or administrative (`root` / `Administrator`) compromise of the developer's workstation.

---

## Reporting a Vulnerability Privately

**Please DO NOT report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

Instead, please report suspected vulnerabilities privately via **GitHub Security Advisories**:

1. Navigate to [**Report a Vulnerability on GitHub**](https://github.com/yogaardiansyah/rakitray/security/advisories/new).
2. Provide a clear description of the vulnerability, the affected component (`src-tauri/supervisor`, `src-tauri/extractor`, `src-tauri/config`, etc.), and a minimal Proof of Concept (PoC) such as a sample `.dev.toml` or reproduction steps.
3. Include your operating system and Rakitray version.

### Response Timeline
- **Initial Acknowledgment:** Within **48 hours** of submission.
- **Triage & Severity Assessment:** Within **5 business days**.
- **Patch & Coordinated Disclosure:** Critical/High severity fixes are prioritized for an immediate patch release, with full credit given to the reporter in the security advisory and `CHANGELOG.md` (unless anonymity is requested).

