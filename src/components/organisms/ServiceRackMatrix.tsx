import { For, Show, createMemo, createSignal } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";
import { MasterControlStrip, ServiceRow } from "../molecules";
import { EmptyRackOnboarding } from "./EmptyRackOnboarding";

type RackFilter = "all" | "running" | "web_server" | "runtime" | "database" | "cache";

const RACK_FILTERS: { id: RackFilter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "running", label: "Running" },
  { id: "web_server", label: "Web" },
  { id: "runtime", label: "PHP" },
  { id: "database", label: "Database" },
  { id: "cache", label: "Cache" },
];

export function ServiceRackMatrix() {
  const { state, clearError } = useSupervisor();
  const [filter, setFilter] = createSignal<RackFilter>("all");
  const [query, setQuery] = createSignal("");

  const filteredInstances = createMemo(() => {
    const q = query().trim().toLowerCase();
    const f = filter();
    return state.instances.filter((inst) => {
      if (f === "running" && inst.status !== "running") return false;
      if (f !== "all" && f !== "running" && inst.category !== f) return false;
      if (!q) return true;
      return (
        inst.displayName.toLowerCase().includes(q) ||
        inst.engine.toLowerCase().includes(q) ||
        inst.version.toLowerCase().includes(q) ||
        String(inst.port ?? "").includes(q)
      );
    });
  });

  return (
    <div class="service-rack-matrix">
      <MasterControlStrip />

      <Show when={state.lastError}>
        {(err) => (
          <div class="rack-error-banner font-mono">
            <span class="banner-inline-msg">
              <Icon name="alert" size={14} />
              <span>{err()}</span>
            </span>
            <button type="button" class="rack-error-close" onClick={clearError} title="Dismiss">
              <Icon name="x" size={13} />
            </button>
          </div>
        )}
      </Show>

      <Show when={state.instances.length > 0}>
        <div class="rack-filter-toolbar">
          <div class="rack-filter-pills">
            <For each={RACK_FILTERS}>
              {(item) => (
                <button
                  type="button"
                  class={`rack-filter-pill ${filter() === item.id ? "active" : ""}`}
                  onClick={() => setFilter(item.id)}
                >
                  {item.label}
                </button>
              )}
            </For>
          </div>
          <div class="rack-search-wrap">
            <Icon name="search" size={13} class="rack-search-icon" />
            <input
              type="search"
              class="rack-search-input font-mono"
              placeholder="Filter services..."
              aria-label="Filter services by name, version, or port"
              value={query()}
              onInput={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
        </div>
      </Show>

      <div class="dense-matrix-container">
        {/* Sticky Table Header */}
        <div class="dense-table-header">
          <div class="col-service">SERVICE / ENGINE</div>
          <div class="col-version">VERSION & PROFILE</div>
          <div class="col-status">STATUS</div>
          <div class="col-network">PID / PORT</div>
          <div class="col-resources">CPU / MEM</div>
          <div class="col-actions">ACTIONS</div>
        </div>

        <Show when={state.instances.length > 0} fallback={<EmptyRackOnboarding />}>
          <div class="dense-table-body">
            <For
              each={filteredInstances()}
              fallback={
                <div class="rack-empty-filter font-mono">No services match the current filter.</div>
              }
            >
              {(instance) => <ServiceRow instance={instance} />}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
}
