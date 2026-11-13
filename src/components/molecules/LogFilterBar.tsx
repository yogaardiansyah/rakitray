import { For } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";

interface LogFilterBarProps {
  collapsed?: boolean;
  onToggleCollapse?: () => void;
}

export function LogFilterBar(props: LogFilterBarProps) {
  const { state, setLogFilter, clearLogs } = useSupervisor();

  return (
    <div class="dense-log-filter-bar">
      <div class="filter-pills-scroll no-scrollbar">
        <button
          type="button"
          class={`dense-filter-pill ${state.logFilter === "all" ? "active" : ""}`}
          onClick={() => setLogFilter("all")}
        >
          All ({state.logs.length})
        </button>

        <For each={state.instances}>
          {(inst) => (
            <button
              type="button"
              class={`dense-filter-pill ${state.logFilter === inst.instanceId ? "active" : ""}`}
              onClick={() => setLogFilter(inst.instanceId)}
            >
              {inst.displayName}
            </button>
          )}
        </For>
      </div>

      <div class="filter-controls-pinned">
        <span class="batch-indicator font-mono">Live</span>
        <button
          type="button"
          class="dense-action-btn btn-clear-logs"
          onClick={clearLogs}
          title="Clear log buffer"
        >
          Clear
        </button>
        {props.onToggleCollapse && (
          <button
            type="button"
            class="dense-action-btn btn-clear-logs"
            onClick={props.onToggleCollapse}
            title={props.collapsed ? "Expand terminal" : "Collapse terminal"}
          >
            <Icon name={props.collapsed ? "chevronUp" : "chevronDown"} size={13} />
            <span>{props.collapsed ? "Expand" : "Collapse"}</span>
          </button>
        )}
      </div>
    </div>
  );
}
