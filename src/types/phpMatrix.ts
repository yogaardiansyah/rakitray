export type PhpProfileMode = "dev" | "prod";

export type ExtensionKind = "extension" | "zendExtension";

export type ExtensionSource = "sharedLib" | "builtIn";

export type ExtensionCategory =
  | "laravelCore"
  | "databaseCache"
  | "performanceDebug"
  | "mediaFormat"
  | "other";

export interface PhpExtension {
  name: string;
  kind: ExtensionKind;
  source: ExtensionSource;
  category: ExtensionCategory;
  enabled: boolean;
  toggleable: boolean;
  requires: string[];
}

export interface PhpIniLimits {
  memoryLimit: string;
  uploadMaxFilesize: string;
  postMaxSize: string;
  maxExecutionTime: number;
}

export interface PhpIniDirective {
  key: string;
  value: string;
  enabled: boolean;
  section: string;
  custom: boolean;
}

export interface PhpMatrix {
  instanceId: string;
  version: string;
  profile: PhpProfileMode;
  limits: PhpIniLimits;
  extensions: PhpExtension[];
  directives: PhpIniDirective[];
  warnings: string[];
}

export type PhpMatrixEvent =
  | { status: "applying"; instanceId: string; version: string }
  | { status: "reloaded"; instanceId: string; version: string }
  | { status: "rolledBack"; instanceId: string; version: string; reason: string };
