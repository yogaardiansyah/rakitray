import { type Channel, invoke } from "@tauri-apps/api/core";
import type {
  PhpIniDirective,
  PhpIniLimits,
  PhpMatrix,
  PhpMatrixEvent,
  PhpProfileMode,
} from "../types/phpMatrix";

export async function subscribePhpMatrix(onEvent: Channel<PhpMatrixEvent>): Promise<void> {
  return invoke<void>("subscribe_php_matrix", { onEvent });
}

export async function getPhpMatrix(instanceId: string): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("get_php_matrix", { instanceId });
}

export async function togglePhpExtension(
  instanceId: string,
  extName: string,
  enabled: boolean,
): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("toggle_php_extension", {
    instanceId,
    extName,
    enabled,
  });
}

export async function applyPhpProfile(
  instanceId: string,
  profile: PhpProfileMode,
  limits: PhpIniLimits,
): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("apply_php_profile", {
    instanceId,
    profile,
    limits,
  });
}

export async function savePhpIniDirectives(
  instanceId: string,
  directives: PhpIniDirective[],
): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("save_php_ini_directives", {
    instanceId,
    directives,
  });
}

export async function applyLaravelPreset(instanceId: string): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("apply_laravel_preset", { instanceId });
}

export async function clonePhpMatrix(
  sourceInstanceId: string,
  targetInstanceId: string,
): Promise<PhpMatrix> {
  return invoke<PhpMatrix>("clone_php_matrix", {
    sourceInstanceId,
    targetInstanceId,
  });
}
