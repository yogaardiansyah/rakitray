import { For, Show, onMount } from "solid-js";
import { useRakiStore } from "../../store/storeState";
import { useSupervisor } from "../../store/supervisorStore";

export function RakiStorePanel() {
  const { storeState, fetchCatalog, installRuntime, uninstallRuntime, setCategory } =
    useRakiStore();
  const { scanRuntimes } = useSupervisor();

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

  const filteredItems = () => {
    if (storeState.selectedCategory === "all") return storeState.catalog;
    return storeState.catalog.filter((i) => i.entry.category === storeState.selectedCategory);
  };

  const handleInstall = async (catalogId: string) => {
    await installRuntime(catalogId, async () => {
      await scanRuntimes();
    });
  };

  return (
    <div class="rakistore-panel p-6 max-w-6xl mx-auto">
      {/* Header */}
      <div class="flex items-center justify-between mb-6">
        <div>
          <h2 class="text-xl font-bold text-slate-100">RakiStore Runtime Store</h2>
          <p class="text-xs text-slate-400 mt-1">
            Install and manage pre-compiled engine binaries with zero-bloat lean extraction.
          </p>
        </div>
        <button
          type="button"
          class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs rounded border border-slate-700 transition"
          onClick={() => fetchCatalog(true)}
        >
          Refresh Catalog
        </button>
      </div>

      {/* Category Pills */}
      <div class="flex gap-2 mb-6 border-b border-slate-800 pb-3">
        <For each={categories}>
          {(cat) => (
            <button
              type="button"
              class={`px-3 py-1 text-xs rounded-full transition ${
                storeState.selectedCategory === cat.id
                  ? "bg-indigo-600 text-white font-medium"
                  : "bg-slate-800/60 text-slate-400 hover:bg-slate-800"
              }`}
              onClick={() => setCategory(cat.id)}
            >
              {cat.label}
            </button>
          )}
        </For>
      </div>

      {/* Catalog Grid */}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <For each={filteredItems()}>
          {(item) => {
            const progress = () => storeState.activeProgress[item.entry.id];
            const isInstalled = () => item.installed;

            return (
              <div class="package-card bg-slate-900 border border-slate-800 rounded-lg p-4 flex flex-col justify-between">
                <div>
                  <div class="flex items-start justify-between">
                    <div>
                      <h4 class="text-sm font-semibold text-slate-100">{item.entry.engine}</h4>
                      <span class="text-xs text-slate-400">{item.entry.version}</span>
                    </div>
                    <span class="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-slate-800 text-slate-300">
                      ~{item.entry.estimatedSizeMb} MB
                    </span>
                  </div>

                  {/* Progress Indicator */}
                  <Show when={progress()}>
                    {(p) => (
                      <div class="mt-3 p-2 bg-slate-950 rounded border border-slate-800 text-xs">
                        <div class="flex justify-between text-[11px] text-slate-400 mb-1">
                          <span class="capitalize">{p().state.replace("_", " ")}</span>
                          <span>
                            {p().speedBps > 0 ? `${Math.round(p().speedBps / 1024)} KB/s` : ""}
                          </span>
                        </div>
                        <div class="w-full bg-slate-800 h-1.5 rounded-full overflow-hidden">
                          <div
                            class="bg-indigo-500 h-full transition-all duration-150"
                            style={{
                              width: `${
                                p().totalBytes > 0
                                  ? Math.min(
                                      100,
                                      Math.round((p().bytesDownloaded / p().totalBytes) * 100),
                                    )
                                  : 0
                              }%`,
                            }}
                          />
                        </div>
                        <Show when={p().message}>
                          <span class="text-[10px] text-emerald-400 mt-1 block truncate">
                            {p().message}
                          </span>
                        </Show>
                      </div>
                    )}
                  </Show>
                </div>

                <div class="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-between">
                  <span
                    class={`text-[11px] font-medium ${isInstalled() ? "text-emerald-400" : "text-slate-500"}`}
                  >
                    {isInstalled() ? "✓ Installed" : "Available"}
                  </span>

                  <Show
                    when={isInstalled()}
                    fallback={
                      <button
                        type="button"
                        class="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium rounded transition"
                        onClick={() => handleInstall(item.entry.id)}
                      >
                        Install
                      </button>
                    }
                  >
                    <button
                      type="button"
                      class="px-2.5 py-1 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 text-xs rounded transition border border-rose-500/20"
                      onClick={() => uninstallRuntime(item.entry.id)}
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
