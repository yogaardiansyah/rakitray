export type ServiceCategory = "web_server" | "runtime" | "database" | "cache" | "tool";

export type ServiceStatus = "stopped" | "starting" | "running" | "stopping" | "crashed";

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

export interface DiscoveredRuntime {
  category: ServiceCategory;
  engine: string;
  version: string;
  binaryPath: string;
  isValid: boolean;
}

export interface ProjectSite {
  name: string;
  domain: string;
  rootPath: string;
  webServerId: string;
  runtimeId: string;
  port: number;
}

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
