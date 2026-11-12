import { Channel, invoke } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import type {
  DiscoveredRuntime,
  IpcStreamEvent,
  LogEntry,
  ProjectSite,
  ServiceInstance,
} from "../types/ipc";

const [instances, setInstances] = createStore<Record<string, ServiceInstance>>({});
const [logs, setLogs] = createStore<LogEntry[]>([]);
const [runtimes, setRuntimes] = createStore<DiscoveredRuntime[]>([]);
const [sites, setSites] = createStore<ProjectSite[]>([]);
const [isSubscribed, setIsSubscribed] = createSignal(false);

const MAX_LOG_LINES = 1000;

export function useSupervisorStore() {
  async function init() {
    if (isSubscribed()) return;

    try {
      const channel = new Channel<IpcStreamEvent>();
      channel.onmessage = (event: IpcStreamEvent) => {
        if (event.type === "telemetryUpdate") {
          for (const inst of event.payload) {
            setInstances(inst.instanceId, inst);
          }
        } else if (event.type === "logBatch") {
          setLogs((prev) => {
            const next = [...prev, ...event.payload];
            return next.length > MAX_LOG_LINES ? next.slice(next.length - MAX_LOG_LINES) : next;
          });
        }
      };

      await invoke("subscribe_supervisor", { onEvent: channel });
      setIsSubscribed(true);

      const discovered = await invoke<DiscoveredRuntime[]>("scan_installed_runtimes");
      setRuntimes(discovered);
    } catch (err) {
      console.warn("Failed to subscribe to supervisor channel:", err);
    }
  }

  async function startInstance(instanceId: string) {
    try {
      const updated = await invoke<ServiceInstance>("start_instance", {
        instanceId,
      });
      setInstances(updated.instanceId, updated);
    } catch (err) {
      console.error(`Failed to start instance ${instanceId}:`, err);
    }
  }

  async function stopInstance(instanceId: string) {
    try {
      const updated = await invoke<ServiceInstance>("stop_instance", {
        instanceId,
      });
      setInstances(updated.instanceId, updated);
    } catch (err) {
      console.error(`Failed to stop instance ${instanceId}:`, err);
    }
  }

  async function saveSite(site: ProjectSite) {
    try {
      await invoke("save_project_site", { site });
      setSites((prev) => {
        const idx = prev.findIndex((s) => s.name === site.name);
        if (idx >= 0) {
          const clone = [...prev];
          clone[idx] = site;
          return clone;
        }
        return [...prev, site];
      });
    } catch (err) {
      console.error("Failed to save project site:", err);
    }
  }

  function clearLogs() {
    setLogs([]);
  }

  return {
    instances,
    logs,
    runtimes,
    sites,
    init,
    startInstance,
    stopInstance,
    saveSite,
    clearLogs,
  };
}
