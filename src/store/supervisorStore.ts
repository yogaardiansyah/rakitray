import { Channel, invoke } from "@tauri-apps/api/core";
import { createStore, produce } from "solid-js/store";
import type {
  DiscoveredRuntime,
  IpcStreamEvent,
  LogEntry,
  ProjectSite,
  ServiceInstance,
} from "../types/ipc";

export type ActiveTab = "rack" | "sites" | "runtimes";

export interface SupervisorState {
  instances: ServiceInstance[];
  discoveredRuntimes: DiscoveredRuntime[];
  logs: LogEntry[];
  sites: ProjectSite[];
  activeTab: ActiveTab;
  logFilter: string;
  isScanning: boolean;
  isSubscribed: boolean;
}

const MAX_LOGS = 500;

const [state, setState] = createStore<SupervisorState>({
  instances: [],
  discoveredRuntimes: [],
  logs: [],
  sites: [],
  activeTab: "rack",
  logFilter: "all",
  isScanning: false,
  isSubscribed: false,
});

export function useSupervisor() {
  async function initSupervisorChannel() {
    if (state.isSubscribed) return;

    try {
      const channel = new Channel<IpcStreamEvent>();
      channel.onmessage = (event: IpcStreamEvent) => {
        if (event.type === "telemetryUpdate") {
          setState("instances", event.payload);
        } else if (event.type === "logBatch") {
          setState(
            "logs",
            produce((draft) => {
              for (const entry of event.payload) {
                draft.push(entry);
              }
              if (draft.length > MAX_LOGS) {
                draft.splice(0, draft.length - MAX_LOGS);
              }
            }),
          );
        }
      };

      await invoke("subscribe_supervisor", { onEvent: channel });
      setState("isSubscribed", true);
      await scanRuntimes();
    } catch (err) {
      console.warn("Failed to subscribe to supervisor channel:", err);
    }
  }

  async function scanRuntimes() {
    setState("isScanning", true);
    try {
      const runtimes = await invoke<DiscoveredRuntime[]>("scan_installed_runtimes");
      setState("discoveredRuntimes", runtimes);
    } catch (err) {
      console.error("Failed to scan installed runtimes:", err);
    } finally {
      setState("isScanning", false);
    }
  }

  async function startService(instanceId: string) {
    try {
      const updated = await invoke<ServiceInstance>("start_instance", {
        instanceId,
      });
      setState("instances", (inst) => inst.instanceId === instanceId, updated);
    } catch (err) {
      console.error(`Failed to start service ${instanceId}:`, err);
    }
  }

  async function stopService(instanceId: string) {
    try {
      const updated = await invoke<ServiceInstance>("stop_instance", {
        instanceId,
      });
      setState("instances", (inst) => inst.instanceId === instanceId, updated);
    } catch (err) {
      console.error(`Failed to stop service ${instanceId}:`, err);
    }
  }

  async function startAllServices() {
    const stopped = state.instances.filter((i) => i.status === "stopped");
    for (const inst of stopped) {
      await startService(inst.instanceId);
    }
  }

  async function stopAllServices() {
    const running = state.instances.filter((i) => i.status === "running");
    for (const inst of running) {
      await stopService(inst.instanceId);
    }
  }

  async function saveProjectSite(site: ProjectSite): Promise<boolean> {
    try {
      await invoke("save_project_site", { site });
      setState(
        "sites",
        produce((draft) => {
          const idx = draft.findIndex((s) => s.name === site.name);
          if (idx >= 0) {
            draft[idx] = site;
          } else {
            draft.push(site);
          }
        }),
      );
      return true;
    } catch (err) {
      console.error("Failed to save project site:", err);
      return false;
    }
  }

  function clearLogs() {
    setState("logs", []);
  }

  function setActiveTab(tab: ActiveTab) {
    setState("activeTab", tab);
  }

  function setLogFilter(filter: string) {
    setState("logFilter", filter);
  }

  return {
    state,
    initSupervisorChannel,
    scanRuntimes,
    startService,
    stopService,
    startAllServices,
    stopAllServices,
    saveProjectSite,
    clearLogs,
    setActiveTab,
    setLogFilter,
  };
}
