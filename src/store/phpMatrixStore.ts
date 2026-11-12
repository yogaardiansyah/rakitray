import { Channel } from "@tauri-apps/api/core";
import { createStore, produce } from "solid-js/store";
import {
  applyLaravelPreset as apiApplyLaravelPreset,
  applyPhpProfile as apiApplyPhpProfile,
  clonePhpMatrix as apiClonePhpMatrix,
  getPhpMatrix as apiGetPhpMatrix,
  savePhpIniDirectives as apiSavePhpIniDirectives,
  subscribePhpMatrix as apiSubscribePhpMatrix,
  togglePhpExtension as apiTogglePhpExtension,
} from "../lib/phpMatrixApi";
import type {
  PhpIniDirective,
  PhpIniLimits,
  PhpMatrix,
  PhpMatrixEvent,
  PhpProfileMode,
} from "../types/phpMatrix";

export type MatrixApplyStatus = "idle" | "applying" | "reloaded" | "rolledBack";

export interface PhpMatrixStoreState {
  selectedInstanceId: string;
  matrices: Record<string, PhpMatrix>;
  pendingProfile: PhpProfileMode | null;
  pendingLimits: PhpIniLimits | null;
  applyStatus: MatrixApplyStatus;
  lastRollbackReason: string | null;
  warnings: string[];
  error: string | null;
  isLoading: boolean;
  isSubscribed: boolean;
}

const DEFAULT_DEV_LIMITS: PhpIniLimits = {
  memoryLimit: "512M",
  uploadMaxFilesize: "128M",
  postMaxSize: "128M",
  maxExecutionTime: 300,
};

const DEFAULT_PROD_LIMITS: PhpIniLimits = {
  memoryLimit: "256M",
  uploadMaxFilesize: "32M",
  postMaxSize: "32M",
  maxExecutionTime: 60,
};

const [state, setState] = createStore<PhpMatrixStoreState>({
  selectedInstanceId: "",
  matrices: {},
  pendingProfile: null,
  pendingLimits: null,
  applyStatus: "idle",
  lastRollbackReason: null,
  warnings: [],
  error: null,
  isLoading: false,
  isSubscribed: false,
});

let statusTimeoutId: ReturnType<typeof setTimeout> | null = null;

function setTimedApplyStatus(status: MatrixApplyStatus, durationMs = 4000) {
  if (statusTimeoutId) {
    clearTimeout(statusTimeoutId);
    statusTimeoutId = null;
  }
  setState("applyStatus", status);
  if (status !== "idle" && status !== "applying") {
    statusTimeoutId = setTimeout(() => {
      setState("applyStatus", "idle");
    }, durationMs);
  }
}

export function usePhpMatrix() {
  async function initMatrixChannel() {
    if (state.isSubscribed) return;

    try {
      const channel = new Channel<PhpMatrixEvent>();
      channel.onmessage = (event: PhpMatrixEvent) => {
        if (event.status === "applying") {
          setTimedApplyStatus("applying");
          setState("lastRollbackReason", null);
        } else if (event.status === "reloaded") {
          setTimedApplyStatus("reloaded");
          setState("lastRollbackReason", null);
          if (state.selectedInstanceId) {
            void fetchMatrix(state.selectedInstanceId, false);
          }
        } else if (event.status === "rolledBack") {
          setTimedApplyStatus("rolledBack", 8000);
          setState("lastRollbackReason", event.reason);
          if (state.selectedInstanceId) {
            void fetchMatrix(state.selectedInstanceId, false);
          }
        }
      };

      await apiSubscribePhpMatrix(channel);
      setState("isSubscribed", true);
    } catch (err) {
      console.warn("Failed to subscribe to PHP Matrix channel:", err);
    }
  }

  async function fetchMatrix(instanceId: string, showLoading = true) {
    if (!instanceId) return;
    if (showLoading) setState("isLoading", true);
    try {
      const matrix = await apiGetPhpMatrix(instanceId);
      setState(
        produce((draft) => {
          draft.matrices[instanceId] = matrix;
          draft.warnings = matrix.warnings;
          draft.error = null;
        }),
      );
    } catch (err) {
      setState("error", String(err));
    } finally {
      if (showLoading) setState("isLoading", false);
    }
  }

  async function selectInstance(instanceId: string) {
    setState(
      produce((draft) => {
        draft.selectedInstanceId = instanceId;
        draft.pendingProfile = null;
        draft.pendingLimits = null;
        draft.error = null;
        draft.lastRollbackReason = null;
      }),
    );

    if (instanceId) {
      await fetchMatrix(instanceId);
    }
  }

  async function toggleExtension(extName: string, enabled: boolean) {
    const id = state.selectedInstanceId;
    if (!id) return;

    const currentMatrix = state.matrices[id];
    if (!currentMatrix) return;

    // Optimistic UI update
    setState(
      "matrices",
      id,
      "extensions",
      produce((draft) => {
        const ext = draft.find((e) => e.name === extName);
        if (ext) ext.enabled = enabled;
      }),
    );

    try {
      const updated = await apiTogglePhpExtension(id, extName, enabled);
      setState(
        produce((draft) => {
          draft.matrices[id] = updated;
          draft.warnings = updated.warnings;
          draft.error = null;
        }),
      );
    } catch (err) {
      // Revert on error
      setState(
        "matrices",
        id,
        "extensions",
        produce((draft) => {
          const ext = draft.find((e) => e.name === extName);
          if (ext) ext.enabled = !enabled;
        }),
      );
      setState("error", String(err));
    }
  }

  function stageProfile(profile: PhpProfileMode) {
    setState("pendingProfile", profile);
    if (!state.pendingLimits) {
      setState(
        "pendingLimits",
        profile === "dev" ? { ...DEFAULT_DEV_LIMITS } : { ...DEFAULT_PROD_LIMITS },
      );
    }
  }

  function stageLimit(field: keyof PhpIniLimits, value: string | number) {
    const current = state.pendingLimits ??
      state.matrices[state.selectedInstanceId]?.limits ?? {
        ...DEFAULT_DEV_LIMITS,
      };

    setState("pendingLimits", {
      ...current,
      [field]: value,
    });
  }

  function discardPending() {
    setState(
      produce((draft) => {
        draft.pendingProfile = null;
        draft.pendingLimits = null;
      }),
    );
  }

  async function commitPending() {
    const id = state.selectedInstanceId;
    if (!id) return;

    const currentMatrix = state.matrices[id];
    if (!currentMatrix) return;

    const profile = state.pendingProfile ?? currentMatrix.profile;
    const limits = state.pendingLimits ?? currentMatrix.limits;

    setState("isLoading", true);
    setState("error", null);

    try {
      const updated = await apiApplyPhpProfile(id, profile, limits);
      setState(
        produce((draft) => {
          draft.matrices[id] = updated;
          draft.warnings = updated.warnings;
          draft.pendingProfile = null;
          draft.pendingLimits = null;
          draft.error = null;
        }),
      );
    } catch (err) {
      setState("error", String(err));
    } finally {
      setState("isLoading", false);
    }
  }

  async function runLaravelPreset() {
    const id = state.selectedInstanceId;
    if (!id) return;

    setState("isLoading", true);
    setState("error", null);

    try {
      const updated = await apiApplyLaravelPreset(id);
      setState(
        produce((draft) => {
          draft.matrices[id] = updated;
          draft.warnings = updated.warnings;
          draft.error = null;
        }),
      );
    } catch (err) {
      setState("error", String(err));
    } finally {
      setState("isLoading", false);
    }
  }

  async function cloneToTarget(targetInstanceId: string) {
    const sourceId = state.selectedInstanceId;
    if (!sourceId || !targetInstanceId || sourceId === targetInstanceId) return;

    setState("isLoading", true);
    setState("error", null);

    try {
      const updatedTarget = await apiClonePhpMatrix(sourceId, targetInstanceId);
      setState(
        produce((draft) => {
          draft.matrices[targetInstanceId] = updatedTarget;
          draft.selectedInstanceId = targetInstanceId;
          draft.pendingProfile = null;
          draft.pendingLimits = null;
          draft.warnings = updatedTarget.warnings;
          draft.error = null;
        }),
      );
    } catch (err) {
      setState("error", String(err));
    } finally {
      setState("isLoading", false);
    }
  }

  async function saveDirectives(newDirectives: PhpIniDirective[]) {
    const id = state.selectedInstanceId;
    if (!id) return;

    setState("isLoading", true);
    setState("error", null);

    try {
      const updated = await apiSavePhpIniDirectives(id, newDirectives);
      setState(
        produce((draft) => {
          draft.matrices[id] = updated;
          draft.warnings = updated.warnings;
          draft.error = null;
        }),
      );
    } catch (err) {
      setState("error", String(err));
    } finally {
      setState("isLoading", false);
    }
  }

  async function toggleDirective(key: string, enabledOverride?: boolean) {
    const id = state.selectedInstanceId;
    if (!id) return;

    const current = state.matrices[id];
    if (!current?.directives) return;

    const target = current.directives.find((d) => d.key === key);
    if (!target) return;

    const nextEnabled = enabledOverride !== undefined ? enabledOverride : !target.enabled;
    const newDirectives = current.directives.map((d) =>
      d.key === key ? { ...d, enabled: nextEnabled } : d,
    );

    await saveDirectives(newDirectives);
  }

  async function updateDirectiveValue(key: string, value: string) {
    const id = state.selectedInstanceId;
    if (!id) return;

    const current = state.matrices[id];
    if (!current?.directives) return;

    const newDirectives = current.directives.map((d) => (d.key === key ? { ...d, value } : d));

    await saveDirectives(newDirectives);
  }

  async function addCustomDirective(key: string, value: string, section = "Custom") {
    const id = state.selectedInstanceId;
    if (!id) return;

    const current = state.matrices[id];
    if (!current?.directives) return;

    const trimmedKey = key.trim();
    if (!trimmedKey) return;

    const existingIdx = current.directives.findIndex((d) => d.key === trimmedKey);
    let newDirectives: PhpIniDirective[];

    if (existingIdx >= 0) {
      newDirectives = current.directives.map((d) =>
        d.key === trimmedKey ? { ...d, value, enabled: true } : d,
      );
    } else {
      newDirectives = [
        ...current.directives,
        {
          key: trimmedKey,
          value: value.trim(),
          enabled: true,
          section,
          custom: true,
        },
      ];
    }

    await saveDirectives(newDirectives);
  }

  async function removeCustomDirective(key: string) {
    const id = state.selectedInstanceId;
    if (!id) return;

    const current = state.matrices[id];
    if (!current?.directives) return;

    const newDirectives = current.directives.filter((d) => d.key !== key);
    await saveDirectives(newDirectives);
  }

  function clearError() {
    setState("error", null);
  }

  function clearWarnings() {
    setState("warnings", []);
  }

  return {
    state,
    initMatrixChannel,
    selectInstance,
    fetchMatrix,
    toggleExtension,
    saveDirectives,
    toggleDirective,
    updateDirectiveValue,
    addCustomDirective,
    removeCustomDirective,
    stageProfile,
    stageLimit,
    discardPending,
    commitPending,
    runLaravelPreset,
    cloneToTarget,
    clearError,
    clearWarnings,
  };
}
