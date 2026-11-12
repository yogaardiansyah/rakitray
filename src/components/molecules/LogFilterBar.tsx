import { For } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";

export function LogFilterBar() {
  const { state, setLogFilter, clearLogs } = useSupervisor();

  return (
    <div class="dense-log-filter-bar">
      <div class="filter-pills-scroll no-scrollbar">
        <button
          type="button"
          class={`dense-filter-pill ${state.logFilter === "all" ? "active" : ""}`}
          onClick={() => setLogFilter("all")}
        >
          All Streams
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
        <span class="batch-indicator font-mono">⚡ 16ms Micro-Batch</span>
        <button
          type="button"
          class="dense-action-btn btn-clear-logs"
          onClick={clearLogs}
          title="Clear log buffer"
        >
          Clear Logs
        </button>
      </div>
    </div>
  );
}
