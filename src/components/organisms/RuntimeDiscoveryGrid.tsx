import { For } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { ActionButton, StatusLed } from "../atoms";

export function RuntimeDiscoveryGrid() {
  const { state, scanRuntimes } = useSupervisor();

  return (
    <div class="runtime-discovery-grid card">
      <div class="panel-header">
        <div>
          <h3>Portable Runtime Catalog</h3>
          <p class="subtitle">
            Discovered in local <code>runtime/bin/</code> hierarchy
          </p>
        </div>

        <ActionButton variant="ghost" onClick={scanRuntimes} loading={state.isScanning}>
          ↻ Rescan Directory
        </ActionButton>
      </div>

      <div class="discovery-table">
        <div class="discovery-header">
          <span>STATUS</span>
          <span>CATEGORY</span>
          <span>ENGINE</span>
          <span>VERSION</span>
          <span>BINARY PATH</span>
        </div>

        <For
          each={state.discoveredRuntimes}
          fallback={<div class="empty-state">No portable runtimes found in runtime/bin/</div>}
        >
          {(item) => (
            <div class="discovery-row">
              <span class="status-cell">
                <StatusLed status={item.isValid ? "running" : "crashed"} size="sm" />
                <span class="status-text font-mono text-xs">
                  {item.isValid ? "VALID" : "MISSING"}
                </span>
              </span>
              <span class="category-text">{item.category}</span>
              <span class="engine-text font-mono">{item.engine}</span>
              <span class="version-text font-mono">v{item.version}</span>
              <span class="path-text font-mono text-xs" title={item.binaryPath}>
                {item.binaryPath}
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
