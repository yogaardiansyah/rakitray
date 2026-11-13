import { Match, Switch } from "solid-js";
import { type ActiveTab, useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";
import {
  DockedLogTerminal,
  PhpMatrixPanel,
  RakiStorePanel,
  RuntimeDiscoveryGrid,
  ServiceRackMatrix,
  SidebarNav,
  SiteRoutingPanel,
} from "../organisms";

const TAB_LABELS: Record<ActiveTab, { title: string; subtitle: string }> = {
  rack: {
    title: "Service Rack",
    subtitle: "Manage local servers and runtimes",
  },
  sites: {
    title: "Sites & Routing",
    subtitle: "Local domains, folders, and ports",
  },
  phpMatrix: {
    title: "PHP Matrix",
    subtitle: "Extensions, profiles, and php.ini settings",
  },
  runtimes: {
    title: "Runtime Binaries",
    subtitle: "Installed binaries in runtime/bin",
  },
  store: {
    title: "RakiStore",
    subtitle: "Download and manage local runtimes",
  },
};

export function WorkstationShell() {
  const { state, scanRuntimes, setActiveTab } = useSupervisor();
  const currentMeta = () => TAB_LABELS[state.activeTab];

  return (
    <div class="app-layout select-none">
      <SidebarNav />

      <div class="main-workspace">
        <header class="workstation-topbar">
          <div class="topbar-breadcrumb">
            <h1 class="topbar-title">{currentMeta().title}</h1>
            <span class="topbar-subtitle">{currentMeta().subtitle}</span>
          </div>

          <div class="topbar-actions">
            <button
              type="button"
              class="topbar-btn"
              disabled={state.isScanning}
              onClick={scanRuntimes}
              aria-label="Rescan installed runtimes"
            >
              <Icon name="refresh" size={13} />
              <span>{state.isScanning ? "Scanning..." : "Rescan"}</span>
            </button>
            {state.activeTab !== "store" && (
              <button
                type="button"
                class="topbar-btn topbar-btn-primary"
                onClick={() => setActiveTab("store")}
              >
                <Icon name="plus" size={13} />
                <span>RakiStore</span>
              </button>
            )}
          </div>
        </header>

        <main class="workspace-content">
          <Switch>
            <Match when={state.activeTab === "rack"}>
              <ServiceRackMatrix />
            </Match>
            <Match when={state.activeTab === "sites"}>
              <SiteRoutingPanel />
            </Match>
            <Match when={state.activeTab === "phpMatrix"}>
              <PhpMatrixPanel />
            </Match>
            <Match when={state.activeTab === "runtimes"}>
              <RuntimeDiscoveryGrid />
            </Match>
            <Match when={state.activeTab === "store"}>
              <RakiStorePanel />
            </Match>
          </Switch>
        </main>

        <DockedLogTerminal />
      </div>
    </div>
  );
}
