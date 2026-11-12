import { Show } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import type { ServiceInstance } from "../../types/ipc";
import { StatusLed, VersionBadge } from "../atoms";

interface ServiceRowProps {
  instance: ServiceInstance;
}

export function ServiceRow(props: ServiceRowProps) {
  const { startService, stopService, setLogFilter } = useSupervisor();

  const isRunning = () => props.instance.status === "running";
  const isTransitioning = () =>
    props.instance.status === "starting" || props.instance.status === "stopping";

  const formatMemory = (bytes: number) => {
    if (!bytes) return "0.0 MB";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getCategoryLabel = () => {
    switch (props.instance.category) {
      case "web_server":
        return "WEB";
      case "runtime":
        return "PHP";
      case "database":
        return "DB";
      case "cache":
        return "CACHE";
      default:
        return "TOOL";
    }
  };

  const getStatusClass = () => {
    switch (props.instance.status) {
      case "running":
        return "status-pill-running";
      case "starting":
      case "stopping":
        return "status-pill-starting";
      case "crashed":
        return "status-pill-crashed";
      default:
        return "status-pill-stopped";
    }
  };

  return (
    <div class="dense-service-row">
      {/* Col 1: Service / Engine */}
      <div class="col-service">
        <StatusLed status={props.instance.status} size="sm" />
        <span class="dense-service-name" title={props.instance.displayName}>
          {props.instance.displayName}
        </span>
        <span class="dense-category-badge">{getCategoryLabel()}</span>
      </div>

      {/* Col 2: Version & Profile */}
      <div class="col-version">
        <VersionBadge
          engine={props.instance.engine}
          version={props.instance.version}
          profile={props.instance.profile}
        />
      </div>

      {/* Col 3: Status */}
      <div class="col-status">
        <span
          class={`dense-status-pill font-mono ${getStatusClass()}`}
          title={props.instance.lastError ?? props.instance.status}
        >
          {props.instance.status.toUpperCase()}
        </span>
      </div>

      {/* Col 4: PID / Port */}
      <div class="col-network font-mono">
        <span class="network-tag">
          <span class="dim">PID:</span> {props.instance.pid ?? "—"}
        </span>
        <span class="network-tag">
          <span class="dim">PORT:</span> {props.instance.port ?? "—"}
        </span>
      </div>

      {/* Col 5: CPU / MEM */}
      <div class="col-resources font-mono">
        <span class="resource-tag">
          <span class="dim">CPU:</span> {props.instance.cpuPercent.toFixed(1)}%
        </span>
        <span class="resource-tag">
          <span class="dim">RAM:</span> {formatMemory(props.instance.memoryBytes)}
        </span>
      </div>

      {/* Col 6: Actions */}
      <div class="col-actions">
        <button
          type="button"
          class="dense-action-btn btn-ghost-icon"
          onClick={() => setLogFilter(props.instance.instanceId)}
          title={`Filter logs for ${props.instance.displayName}`}
        >
          📄
        </button>

        <Show
          when={!isRunning()}
          fallback={
            <button
              type="button"
              class="dense-action-btn btn-danger-solid"
              disabled={isTransitioning()}
              onClick={() => stopService(props.instance.instanceId)}
            >
              Stop
            </button>
          }
        >
          <button
            type="button"
            class="dense-action-btn btn-emerald-solid"
            disabled={isTransitioning()}
            onClick={() => startService(props.instance.instanceId)}
          >
            Start
          </button>
        </Show>
      </div>
    </div>
  );
}
