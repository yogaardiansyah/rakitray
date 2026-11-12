import { Show } from "solid-js";
import type { ServiceInstance } from "../types/ipc";

interface ServiceCardProps {
  instance: ServiceInstance;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
}

export function ServiceCard(props: ServiceCardProps) {
  const isRunning = () => props.instance.status === "running";
  const isTransitioning = () =>
    props.instance.status === "starting" || props.instance.status === "stopping";

  const formatMemory = (bytes: number) => {
    if (!bytes) return "0 MB";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getStatusColor = () => {
    switch (props.instance.status) {
      case "running":
        return "#10b981";
      case "starting":
      case "stopping":
        return "#f59e0b";
      case "crashed":
        return "#ef4444";
      default:
        return "#64748b";
    }
  };

  return (
    <div class="service-card">
      <div class="service-header">
        <div>
          <h4>{props.instance.displayName}</h4>
          <span class="engine-tag">
            {props.instance.engine} v{props.instance.version}
          </span>
        </div>
        <span class="status-indicator" style={{ "background-color": getStatusColor() }}>
          {props.instance.status}
        </span>
      </div>

      <div class="metrics-grid">
        <div class="metric">
          <span class="metric-label">PORT</span>
          <span class="metric-value">{props.instance.port ?? "—"}</span>
        </div>
        <div class="metric">
          <span class="metric-label">PID</span>
          <span class="metric-value">{props.instance.pid ?? "—"}</span>
        </div>
        <div class="metric">
          <span class="metric-label">RAM</span>
          <span class="metric-value">{formatMemory(props.instance.memoryBytes)}</span>
        </div>
        <div class="metric">
          <span class="metric-label">CPU</span>
          <span class="metric-value">{props.instance.cpuPercent.toFixed(1)}%</span>
        </div>
      </div>

      <div class="service-actions">
        <Show
          when={!isRunning()}
          fallback={
            <button
              type="button"
              class="btn btn-danger"
              disabled={isTransitioning()}
              onClick={() => props.onStop(props.instance.instanceId)}
            >
              Stop
            </button>
          }
        >
          <button
            type="button"
            class="btn btn-primary"
            disabled={isTransitioning()}
            onClick={() => props.onStart(props.instance.instanceId)}
          >
            Start
          </button>
        </Show>
      </div>
    </div>
  );
}
