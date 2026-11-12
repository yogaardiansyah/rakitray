import type { DiscoveredRuntime, ServiceCategory } from "./ipc";

export type Platform = "linux-x64" | "linux-arm64" | "windows-x64" | "macos-x64" | "macos-arm64";

export type ArchiveFormat = "tar_gz" | "tar_xz" | "zip";

export interface CatalogEntry {
  id: string;
  engine: string;
  category: ServiceCategory;
  version: string;
  platform: Platform;
  downloadUrl: string;
  sha256: string;
  archiveFormat: ArchiveFormat;
  estimatedSizeMb: number;
  stripComponents: number;
  stripProfile: string;
  blueprintId: string;
}

export interface CatalogItem {
  entry: CatalogEntry;
  installed: boolean;
  installPath: string | null;
}

export type InstallState =
  | "queued"
  | "downloading"
  | "verifying_sha256"
  | "extracting_lean"
  | "registering"
  | "installed"
  | "failed"
  | "cancelled";

export interface StoreErrorInfo {
  code: string;
  detail: string;
}

export interface InstallProgress {
  installId: string;
  catalogId: string;
  state: InstallState;
  bytesDownloaded: number;
  totalBytes: number;
  speedBps: number;
  etaSecs: number | null;
  strippedBytes: number;
  message: string | null;
  error: StoreErrorInfo | null;
}

export type StoreEvent =
  | { type: "progress"; payload: InstallProgress }
  | { type: "installedRuntime"; payload: DiscoveredRuntime };

export interface ConfigPatch {
  port?: number;
  bindAddress?: string;
  profile?: string;
  tunables?: Record<string, string>;
  restart: boolean;
}

export interface DbCreateRequest {
  name: string;
  charset?: string;
  collation?: string;
}

export interface DbUserRequest {
  username: string;
  password: string;
  grantDatabase?: string;
}

export interface DbOverview {
  databases: string[];
  users: string[];
  rootPasswordSet: boolean;
}

export interface StackTemplate {
  id: string;
  name: string;
  description: string;
  members: string[];
}
