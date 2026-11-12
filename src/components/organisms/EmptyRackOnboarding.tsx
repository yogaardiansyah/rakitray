import { useRakiStore } from "../../store/storeState";
import { useSupervisor } from "../../store/supervisorStore";

export function EmptyRackOnboarding() {
  const { setActiveTab, scanRuntimes } = useSupervisor();
  const { installStack } = useRakiStore();

  const handleInstallStack = async (templateId: string) => {
    await installStack(templateId, async () => {
      await scanRuntimes();
    });
  };

  return (
    <div class="empty-rack-onboarding p-6 max-w-4xl mx-auto">
      <div class="text-center mb-8">
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">
          Empty Rack — Quick-Start Stacks
        </h2>
        <p class="text-sm text-slate-400 mt-2">
          Rakitray is running with a clean core. Select a pre-configured 1-click developer stack or
          customize in RakiStore.
        </p>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Laravel Modern Stack */}
        <div class="stack-card bg-slate-900 border border-slate-800 rounded-lg p-5 flex flex-col justify-between hover:border-indigo-500/50 transition">
          <div>
            <div class="flex items-center justify-between mb-3">
              <span class="text-xs font-semibold px-2 py-0.5 rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
                RECOMMENDED
              </span>
              <span class="text-xs text-slate-500">4 Packages</span>
            </div>
            <h3 class="text-lg font-semibold text-slate-100">Laravel Modern</h3>
            <p class="text-xs text-slate-400 mt-1">
              Production-matched environment with high-performance web and cache services.
            </p>
            <div class="mt-4 flex flex-wrap gap-1.5">
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                Nginx 1.26
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                PHP 8.3
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                MariaDB 11.4
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                Redis 7.2
              </span>
            </div>
          </div>
          <button
            type="button"
            class="mt-6 w-full py-2 px-3 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium rounded transition"
            onClick={() => handleInstallStack("laravel-modern")}
          >
            1-Click Install Stack
          </button>
        </div>

        {/* Legacy PHP Stack */}
        <div class="stack-card bg-slate-900 border border-slate-800 rounded-lg p-5 flex flex-col justify-between hover:border-slate-700 transition">
          <div>
            <div class="flex items-center justify-between mb-3">
              <span class="text-xs font-semibold px-2 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700">
                COMPATIBILITY
              </span>
              <span class="text-xs text-slate-500">3 Packages</span>
            </div>
            <h3 class="text-lg font-semibold text-slate-100">Legacy PHP Stack</h3>
            <p class="text-xs text-slate-400 mt-1">
              Optimized for existing PHP 8.2 projects and standard relational databases.
            </p>
            <div class="mt-4 flex flex-wrap gap-1.5">
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                Nginx 1.26
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                PHP 8.2
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                MariaDB 11.4
              </span>
            </div>
          </div>
          <button
            type="button"
            class="mt-6 w-full py-2 px-3 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded transition"
            onClick={() => handleInstallStack("legacy-php")}
          >
            Install PHP 8.2 Stack
          </button>
        </div>

        {/* Custom / RakiStore */}
        <div class="stack-card bg-slate-900 border border-slate-800 rounded-lg p-5 flex flex-col justify-between hover:border-slate-700 transition">
          <div>
            <div class="flex items-center justify-between mb-3">
              <span class="text-xs font-semibold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                CUSTOM
              </span>
              <span class="text-xs text-slate-500">Manual</span>
            </div>
            <h3 class="text-lg font-semibold text-slate-100">Custom Stack</h3>
            <p class="text-xs text-slate-400 mt-1">
              Browse the official RakiStore package catalog and select individual runtimes.
            </p>
            <div class="mt-4 flex flex-wrap gap-1.5">
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">Caddy</span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                PostgreSQL
              </span>
              <span class="text-[11px] bg-slate-800 text-slate-300 px-2 py-0.5 rounded">
                Mailpit
              </span>
            </div>
          </div>
          <button
            type="button"
            class="mt-6 w-full py-2 px-3 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded transition"
            onClick={() => setActiveTab("store")}
          >
            Browse RakiStore →
          </button>
        </div>
      </div>
    </div>
  );
}
