import { For, Show, createMemo, createSignal } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon, StatusLed } from "../atoms";

type HealthFilter = "all" | "valid" | "missing";

export function RuntimeDiscoveryGrid() {
  const { state, setActiveTab } = useSupervisor();
  const [healthFilter, setHealthFilter] = createSignal<HealthFilter>("all");
  const [query, setQuery] = createSignal("");

  const validCount = createMemo(() => state.discoveredRuntimes.filter((r) => r.isValid).length);
  const missingCount = createMemo(() => state.discoveredRuntimes.length - validCount());

  const filteredRuntimes = createMemo(() => {
    const q = query().trim().toLowerCase();
    const hf = healthFilter();
    return state.discoveredRuntimes.filter((item) => {
      if (hf === "valid" && !item.isValid) return false;
      if (hf === "missing" && item.isValid) return false;
      if (!q) return true;
      return (
        item.engine.toLowerCase().includes(q) ||
        item.version.toLowerCase().includes(q) ||
        item.category.toLowerCase().includes(q) ||
        item.binaryPath.toLowerCase().includes(q)
      );
    });
  });

  return (
    <div class="runtime-discovery-grid card">
      <div class="panel-header">
        <div>
          <h3>Portable Runtime Catalog</h3>
          <p class="subtitle">
            Discovered in local <code>runtime/bin/</code> ·{" "}
            <span class="font-mono">
              {validCount()} valid, {missingCount()} missing
            </span>
          </p>
        </div>
      </div>

      <div class="rack-filter-toolbar">
        <div class="rack-filter-pills">
          <button
            type="button"
            class={`rack-filter-pill ${healthFilter() === "all" ? "active" : ""}`}
            onClick={() => setHealthFilter("all")}
          >
            All ({state.discoveredRuntimes.length})
          </button>
          <button
            type="button"
            class={`rack-filter-pill ${healthFilter() === "valid" ? "active" : ""}`}
            onClick={() => setHealthFilter("valid")}
          >
            Valid ({validCount()})
          </button>
          <button
            type="button"
            class={`rack-filter-pill ${healthFilter() === "missing" ? "active" : ""}`}
            onClick={() => setHealthFilter("missing")}
          >
            Missing ({missingCount()})
          </button>
        </div>

        <div class="rack-search-wrap">
          <Icon name="search" size={13} class="rack-search-icon" />
          <input
            type="search"
            class="rack-search-input font-mono"
            placeholder="Filter runtimes..."
            aria-label="Filter discovered runtimes"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
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
          each={filteredRuntimes()}
          fallback={
            <div class="empty-state">
              {state.discoveredRuntimes.length === 0
                ? "No portable runtimes found in runtime/bin/"
                : "No runtimes match the current filter."}
            </div>
          }
        >
          {(item) => (
            <div class="discovery-row">
              <span class="status-cell">
                <StatusLed status={item.isValid ? "running" : "crashed"} size="sm" />
                <span class="status-text font-mono text-xs">
                  {item.isValid ? "Valid" : "Missing"}
                </span>
              </span>
              <span class="category-text">{item.category}</span>
              <span class="engine-text font-mono">{item.engine}</span>
              <span class="version-text font-mono">{item.version}</span>
              <span class="discovery-path-cell">
                <span class="path-text font-mono text-xs" title={item.binaryPath}>
                  {item.binaryPath}
                </span>
                <Show when={!item.isValid}>
                  <button
                    type="button"
                    class="dense-action-btn btn-missing-bin"
                    onClick={() => setActiveTab("store")}
                  >
                    Install
                  </button>
                </Show>
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
