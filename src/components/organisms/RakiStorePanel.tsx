import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import {
  type EngineGroup,
  computeDownloadPercent,
  filterCatalogItems,
  formatTransferRate,
  groupCatalogByEngine,
} from "../../lib/storeHelpers";
import { useRakiStore } from "../../store/storeState";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";

const ENGINE_LABELS: Record<string, string> = {
  mariadb: "MariaDB",
  php: "PHP",
  nginx: "Nginx",
  caddy: "Caddy",
  redis: "Redis",
  mailpit: "Mailpit",
};

function formatEngineName(engine: string): string {
  return ENGINE_LABELS[engine.toLowerCase()] ?? engine;
}

const CATEGORY_LABELS: Record<string, string> = {
  web_server: "Web Server",
  runtime: "Runtime",
  database: "Database",
  cache: "Cache",
  tool: "Tool",
};

function formatCategoryLabel(category: string): string {
  return CATEGORY_LABELS[category] ?? category;
}

export function RakiStorePanel() {
  const {
    storeState,
    fetchCatalog,
    installRuntime,
    cancelInstall,
    installStack,
    uninstallRuntime,
    setCategory,
  } = useRakiStore();
  const { scanRuntimes } = useSupervisor();
  const [searchQuery, setSearchQuery] = createSignal("");
  const [selectedByEngine, setSelectedByEngine] = createSignal<Record<string, string>>({});

  onMount(() => {
    fetchCatalog();
  });

  const categories = [
    { id: "all", label: "All Packages" },
    { id: "web_server", label: "Web Servers" },
    { id: "runtime", label: "Runtimes" },
    { id: "database", label: "Databases" },
    { id: "cache", label: "Cache" },
    { id: "tool", label: "Tools" },
  ];

  const filteredItems = createMemo(() =>
    filterCatalogItems(storeState.catalog, storeState.selectedCategory, searchQuery()),
  );
  const groupedEngines = createMemo(() => groupCatalogByEngine(filteredItems()));

  const getSelectedItem = (group: EngineGroup) => {
    const pickedId = selectedByEngine()[group.engine];
    if (pickedId) {
      const found = group.versions.find((v) => v.entry.id === pickedId);
      if (found) return found;
    }
    return group.versions.find((v) => v.installed) ?? group.versions[0];
  };

  const installedCount = createMemo(() => storeState.catalog.filter((i) => i.installed).length);
  const hasActiveInstall = createMemo(() =>
    Object.values(storeState.activeProgress).some(
      (p) =>
        p.state === "queued" ||
        p.state === "downloading" ||
        p.state === "verifying_sha256" ||
        p.state === "extracting_lean" ||
        p.state === "registering",
    ),
  );

  const handleInstall = async (catalogId: string) => {
    await installRuntime(catalogId, async () => {
      await scanRuntimes();
    });
  };

  return (
    <div class="store-panel">
      <div class="store-header">
        <div>
          <h2 class="store-title">RakiStore Catalog</h2>
          <p class="store-subtitle">Download and manage local server and runtime binaries.</p>
        </div>

        <div class="store-header-actions">
          <div class="store-stats-pill font-mono">
            <span>INSTALLED:</span>
            <strong>
              {installedCount()}/{storeState.catalog.length}
            </strong>
          </div>
          <button
            type="button"
            class="store-refresh-btn"
            disabled={storeState.isLoading}
            onClick={() => fetchCatalog(true)}
          >
            <Icon name="refresh" size={13} />
            <span>{storeState.isLoading ? "Syncing..." : "Refresh Catalog"}</span>
          </button>
        </div>
      </div>

      <div class="store-toolbar">
        <div class="store-category-tabs">
          <For each={categories}>
            {(cat) => (
              <button
                type="button"
                class={`store-category-pill ${storeState.selectedCategory === cat.id ? "active" : ""}`}
                onClick={() => setCategory(cat.id)}
              >
                {cat.label}
              </button>
            )}
          </For>
        </div>

        <input
          type="search"
          class="store-search-input font-mono"
          placeholder="Filter engine or version..."
          aria-label="Filter RakiStore packages"
          value={searchQuery()}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
        />
      </div>

      <Show when={storeState.error}>
        {(err) => <div class="store-error-banner font-mono">{err()}</div>}
      </Show>

      <Show when={storeState.stackTemplates.length > 0}>
        <div class="store-stacks-strip">
          <span class="store-stacks-label font-mono">1-CLICK STACKS</span>
          <div class="store-stacks-list">
            <For each={storeState.stackTemplates}>
              {(tpl) => (
                <button
                  type="button"
                  class="store-stack-pill"
                  disabled={hasActiveInstall() || storeState.isLoading}
                  onClick={() => installStack(tpl.id, () => void scanRuntimes())}
                  title={tpl.description}
                >
                  <span>{tpl.name}</span>
                  <span class="store-stack-count font-mono">{tpl.members.length} pkgs</span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>

      <div class="store-list">
        <div class="store-list-header font-mono">
          <span>ENGINE</span>
          <span>VERSION</span>
          <span>SIZE</span>
          <span>STATUS</span>
          <span class="col-actions">ACTION</span>
        </div>

        <For
          each={groupedEngines()}
          fallback={
            <div class="store-empty-state font-mono">
              No packages match the selected category or filter.
            </div>
          }
        >
          {(group) => {
            const selected = () => getSelectedItem(group);
            const progress = () => storeState.activeProgress[selected().entry.id];
            const isInstalled = () => selected().installed;

            return (
              <div class={`store-list-row ${isInstalled() ? "is-installed" : ""}`}>
                <div class="store-row-engine">
                  <span class="store-engine-name">{formatEngineName(group.engine)}</span>
                  <span class="store-cat-badge">{formatCategoryLabel(group.category)}</span>
                </div>

                <div class="store-row-version">
                  <select
                    class="store-version-select font-mono"
                    aria-label={`${formatEngineName(group.engine)} version`}
                    value={selected().entry.id}
                    onChange={(e) => {
                      const nextId = e.currentTarget.value;
                      setSelectedByEngine((prev) => ({ ...prev, [group.engine]: nextId }));
                    }}
                  >
                    <For each={group.versions}>
                      {(ver) => (
                        <option value={ver.entry.id}>
                          v{ver.entry.version}
                          {ver.installed ? " (Installed)" : ""}
                        </option>
                      )}
                    </For>
                  </select>
                </div>

                <div class="store-row-size font-mono">~{selected().entry.estimatedSizeMb} MB</div>

                <div class="store-row-status">
                  <Show
                    when={progress()}
                    fallback={
                      <span
                        class={`store-status-tag ${isInstalled() ? "is-installed" : "is-available"}`}
                      >
                        <span class="store-status-dot" aria-hidden="true" />
                        {isInstalled() ? "Installed" : "Available"}
                      </span>
                    }
                  >
                    {(p) => (
                      <div class="store-inline-progress">
                        <div class="store-progress-meta font-mono">
                          <span class="store-progress-state">{p().state.replace(/_/g, " ")}</span>
                          <span>{formatTransferRate(p().speedBps)}</span>
                        </div>
                        <div class="store-progress-bar">
                          <div
                            class="store-progress-fill"
                            style={{
                              width: `${computeDownloadPercent(p().bytesDownloaded, p().totalBytes)}%`,
                            }}
                          />
                        </div>
                        <Show when={p().error?.detail ?? p().message}>
                          {(msg) => (
                            <span class="store-progress-msg font-mono" title={msg()}>
                              {msg()}
                            </span>
                          )}
                        </Show>
                      </div>
                    )}
                  </Show>
                </div>

                <div class="store-row-action">
                  <Show
                    when={isInstalled()}
                    fallback={
                      <Show
                        when={
                          progress()?.state === "queued" ||
                          progress()?.state === "downloading" ||
                          progress()?.state === "verifying_sha256" ||
                          progress()?.state === "extracting_lean"
                        }
                        fallback={
                          <button
                            type="button"
                            class="store-btn-install"
                            disabled={hasActiveInstall()}
                            onClick={() => handleInstall(selected().entry.id)}
                          >
                            Install
                          </button>
                        }
                      >
                        <button
                          type="button"
                          class="store-btn-cancel"
                          onClick={() => cancelInstall(selected().entry.id)}
                        >
                          Cancel
                        </button>
                      </Show>
                    }
                  >
                    <button
                      type="button"
                      class="store-btn-uninstall"
                      onClick={() => uninstallRuntime(selected().entry.id)}
                    >
                      Uninstall
                    </button>
                  </Show>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
