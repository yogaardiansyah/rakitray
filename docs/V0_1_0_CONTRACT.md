# Rakitray v0.1.0 Engineering Contract (`docs/V0_1_0_CONTRACT.md`)

This document defines the strict engineering contract, filesystem layout, 1:1 Rust-TypeScript data types, Tauri IPC command signatures, and step-by-step implementation checklist for milestone **`v0.1.0`** (Core CLI Engine, Dynamic Rack Supervisor & Hello Laravel).

---

## 1. Scope of Milestone `v0.1.0`

Milestone `v0.1.0` delivers the fundamental execution core of Rakitray:
1. Dynamic runtime discovery across the portable directory structure (`runtime/bin/<category>/<engine>/<version>/`).
2. Rust process supervisor with anti-zombie guarantees (Linux `prctl` + Unix PGID, Windows Job Objects).
3. Primary drivers for **Nginx**, multi-version **PHP-FPM/CGI**, and **MariaDB**.
4. Standalone `rakit` CLI (`up`, `down`, `status`).
5. SolidJS Dynamic Rack Dashboard with high-frequency telemetry & logs over `tauri::ipc::Channel`.

---

## 2. Directory Layout Contract

All files managed by Rakitray reside strictly inside the self-contained `runtime/` root directory:

```text
runtime/
├── bin/                               # Portable engine binaries discovered dynamically
│   ├── web_server/
│   │   ├── nginx/
│   │   │   └── 1.26.2/
│   │   │       ├── nginx (or nginx.exe)
│   │   │       └── conf/
│   │   └── caddy/
│   │       └── 2.8.4/
│   ├── runtime/
│   │   └── php-cgi/
│   │       ├── 8.2.24/
│   │       │   ├── php-cgi (or php-cgi.exe)
│   │       │   ├── php.ini
│   │       │   └── ext/
│   │       └── 8.3.12/
│   │           ├── php-cgi (or php-cgi.exe)
│   │           ├── php.ini
│   │           └── ext/
│   ├── database/
│   │   └── mariadb/
│   │       └── 11.4.3/
│   │           ├── bin/mariadbd
│   │           └── share/
│   ├── cache/
│   │   └── redis/
│   │       └── 7.2.5/
│   │           └── bin/redis-server
│   └── tool/
│       └── mailpit/
│           └── 1.21.1/
├── config/                            # Dynamically generated runtime configs
│   ├── nginx/
│   │   ├── nginx.conf
│   │   └── sites-enabled/
│   ├── php/
│   │   ├── 8.2-generated.ini
│   │   └── 8.3-generated.ini
│   └── mariadb/
│       └── my.cnf
├── data/                              # Persistent databases & state
│   ├── mariadb/
│   └── redis/
├── logs/                              # Process output log files (*.log)
│   ├── supervisor.log
│   ├── nginx_error.log
│   ├── php_8.3.log
│   └── mariadb.log
├── tmp/                               # Runtime sockets, PID files, named pipes
│   ├── nginx.pid
│   ├── php-8.3.sock
│   └── mariadb.sock
└── www/                               # Default web root and local project sites
    └── default/
        └── index.php
```

---

## 3. Rust & TypeScript 1:1 Data Contracts

All data transferred over Tauri IPC commands and `tauri::ipc::Channel` must strictly conform to these definitions. Rust types use `serde` with `rename_all = "camelCase"` to ensure zero transformation friction with TypeScript.

### 3.1 Service Categories & Status

#### Rust (`src-tauri/src/models/service.rs`)
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCategory {
    WebServer,
    Runtime,
    Database,
    Cache,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
}
```

#### TypeScript (`src/types/service.ts`)
```typescript
export type ServiceCategory =
  | "web_server"
  | "runtime"
  | "database"
  | "cache"
  | "tool";

export type ServiceStatus =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "crashed";
```

---

### 3.2 ServiceInstance

#### Rust (`src-tauri/src/models/instance.rs`)
```rust
use serde::{Deserialize, Serialize};
use super::service::{ServiceCategory, ServiceStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInstance {
    pub instance_id: String,
    pub engine: String,
    pub display_name: String,
    pub category: ServiceCategory,
    pub version: String,
    pub profile: String,
    pub status: ServiceStatus,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub socket_path: Option<String>,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub uptime_secs: u64,
    pub pinned_to_rack: bool,
    pub last_error: Option<String>,
}
```

#### TypeScript (`src/types/service.ts`)
```typescript
export interface ServiceInstance {
  instanceId: string;
  engine: string;
  displayName: string;
  category: ServiceCategory;
  version: string;
  profile: string;
  status: ServiceStatus;
  pid: number | null;
  port: number | null;
  socketPath: string | null;
  memoryBytes: number;
  cpuPercent: number;
  uptimeSecs: number;
  pinnedToRack: boolean;
  lastError: string | null;
}
```

---

### 3.3 ProjectSite

#### Rust (`src-tauri/src/models/project.rs`)
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSite {
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub web_server_id: String,
    pub runtime_id: String,
    pub port: u16,
}
```

#### TypeScript (`src/types/project.ts`)
```typescript
export interface ProjectSite {
  name: string;
  domain: string;
  rootPath: string;
  webServerId: string;
  runtimeId: string;
  port: number;
}
```

---

### 3.4 Discovered Runtime

#### Rust (`src-tauri/src/models/discovery.rs`)
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::service::ServiceCategory;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRuntime {
    pub category: ServiceCategory,
    pub engine: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub is_valid: bool,
}
```

#### TypeScript (`src/types/discovery.ts`)
```typescript
import type { ServiceCategory } from "./service";

export interface DiscoveredRuntime {
  category: ServiceCategory;
  engine: string;
  version: string;
  binaryPath: string;
  isValid: boolean;
}
```

---

### 3.5 High-Frequency IPC Streaming Protocol

Logs and metrics stream from the supervisor to SolidJS using a tagged union over `tauri::ipc::Channel<IpcStreamEvent>`.

#### Rust (`src-tauri/src/models/ipc.rs`)
```rust
use serde::{Deserialize, Serialize};
use super::instance::ServiceInstance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub instance_id: String,
    pub timestamp_ms: u64,
    pub stream: LogStream,
    pub line: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum IpcStreamEvent {
    TelemetryUpdate(Vec<ServiceInstance>),
    LogBatch(Vec<LogEntry>),
}
```

#### TypeScript (`src/types/ipc.ts`)
```typescript
import type { ServiceInstance } from "./service";

export type LogStream = "stdout" | "stderr" | "system";

export interface LogEntry {
  instanceId: string;
  timestampMs: number;
  stream: LogStream;
  line: string;
}

export type IpcStreamEvent =
  | { type: "telemetryUpdate"; payload: ServiceInstance[] }
  | { type: "logBatch"; payload: LogEntry[] };
```

---

## 4. Tauri Commands Contract

The frontend invokes these typed Tauri commands:

| Command | Rust Signature | Description |
| :--- | :--- | :--- |
| `subscribe_supervisor` | `pub fn subscribe_supervisor(on_event: Channel<IpcStreamEvent>, state: State<AppState>) -> Result<(), RakitrayError>` | Establishes the 60Hz telemetry and log push channel. |
| `scan_installed_runtimes` | `pub async fn scan_installed_runtimes(state: State<AppState>) -> Result<Vec<DiscoveredRuntime>, RakitrayError>` | Discovers all available runtimes in `runtime/bin/`. |
| `start_instance` | `pub async fn start_instance(instance_id: String, state: State<AppState>) -> Result<ServiceInstance, RakitrayError>` | Prepares configuration, validates ports, and boots process. |
| `stop_instance` | `pub async fn stop_instance(instance_id: String, state: State<AppState>) -> Result<ServiceInstance, RakitrayError>` | Signals graceful termination (`SIGTERM` -> `SIGKILL`). |
| `update_instance_config` | `pub async fn update_instance_config(instance_id: String, patch: ConfigPatch, state: State<AppState>) -> Result<(), RakitrayError>` | Modifies dynamic configuration directives. |
| `save_project_site` | `pub async fn save_project_site(site: ProjectSite, state: State<AppState>) -> Result<(), RakitrayError>` | Updates local virtual host configuration and routes. |

---

## 5. v0.1.0 Implementation Checklist

### Phase A: Core Models & Type Contracts
- [x] Define Rust models in `src-tauri/src/models/`:
  - `service.rs`: `ServiceCategory`, `ServiceStatus`
  - `instance.rs`: `ServiceInstance`
  - `project.rs`: `ProjectSite`
  - `discovery.rs`: `DiscoveredRuntime`
  - `ipc.rs`: `LogEntry`, `IpcStreamEvent`
- [x] Define mirrored TypeScript types in `src/types/`.

### Phase B: Rust Dynamic Supervisor & Drivers
- [x] Implement filesystem scanner in `src-tauri/src/discovery/` for `runtime/bin/<category>/<engine>/<version>/`.
- [x] Implement the `ServiceDriver` trait in `src-tauri/src/drivers/`.
- [x] Implement drivers:
  - `NginxDriver`: Generates `nginx.conf` and upstreams; monitors port 80/443.
  - `PhpCgiDriver`: Generates `php.ini`; spawns `php-cgi` workers on port/socket.
  - `MariaDbDriver`: Runs initialization (`--initialize-insecure`), generates `my.cnf`, monitors port 3306.
- [x] Implement `Supervisor` in `src-tauri/src/supervisor/`:
  - Process group isolation & Linux `prctl(PR_SET_PDEATHSIG, SIGTERM)`.
  - Windows Win32 Job Object binding with `KILL_ON_JOB_CLOSE`.
  - 16ms micro-batcher dispatching `IpcStreamEvent` over `tauri::ipc::Channel`.

### Phase C: `rakit` CLI Entrypoint
- [x] Create `src-tauri/src/bin/rakit.rs` CLI using `clap` v4.
- [x] Implement CLI subcommands:
  - `rakit up` (boots rack or project stack).
  - `rakit down` (halts running stack).
  - `rakit status` (renders real-time ASCII table or JSON).

### Phase D: SolidJS Dynamic Rack UI & Live Log Channel
- [x] Connect `subscribe_supervisor` in `src/` on mount via Tauri v2 `Channel`.
- [x] Implement `RackDashboard.tsx`: Displays interactive cards for active services (PID, CPU%, RAM, Ports).
- [x] Implement `LogViewer.tsx`: Bounded virtualized log view receiving `LogBatch` events without UI freezing.
- [x] Validate "Hello Laravel" end-to-end execution (Nginx routing to PHP 8.3 + MariaDB).

