import { useRakiStore } from "../store/storeState";
import { type ActiveTab, useSupervisor } from "../store/supervisorStore";

interface McpBridgeRegistry {
  register: (name: string, fn: (...args: unknown[]) => unknown, description: string) => void;
}

declare global {
  interface Window {
    __MCP_BRIDGE__?: McpBridgeRegistry;
  }
}

function registerBridgeHelpers(bridge: McpBridgeRegistry) {
  const { state, setActiveTab, scanRuntimes } = useSupervisor();
  const { storeState, setCategory, fetchCatalog } = useRakiStore();

  bridge.register(
    "getAppState",
    () => ({
      activeTab: state.activeTab,
      instancesCount: state.instances.length,
      runningCount: state.instances.filter((i) => i.status === "running").length,
      discoveredCount: state.discoveredRuntimes.length,
      logFilter: state.logFilter,
      logsCount: state.logs.length,
      selectedCategory: storeState.selectedCategory,
      catalogCount: storeState.catalog.length,
    }),
    "Return snapshot of supervisor & RakiStore state",
  );

  bridge.register(
    "switchTab",
    (tab: unknown) => {
      const valid: ActiveTab[] = ["rack", "sites", "phpMatrix", "runtimes", "store"];
      if (typeof tab === "string" && valid.includes(tab as ActiveTab)) {
        setActiveTab(tab as ActiveTab);
        return { activeTab: state.activeTab };
      }
      throw new Error(`Invalid tab: ${String(tab)}`);
    },
    "Switch active workstation tab (rack|sites|phpMatrix|runtimes|store)",
  );

  bridge.register(
    "setStoreCategory",
    (category: unknown) => {
      if (typeof category === "string") {
        setCategory(category);
        return { selectedCategory: storeState.selectedCategory };
      }
      throw new Error("category must be a string");
    },
    "Filter RakiStore catalog by category",
  );

  bridge.register(
    "refreshAll",
    async () => {
      await Promise.all([scanRuntimes(), fetchCatalog(true)]);
      return {
        discoveredCount: state.discoveredRuntimes.length,
        catalogCount: storeState.catalog.length,
      };
    },
    "Rescan runtimes and refresh RakiStore catalog",
  );
}

export function initMcpBridge() {
  if (typeof window === "undefined") return;
  if (window.__MCP_BRIDGE__) {
    registerBridgeHelpers(window.__MCP_BRIDGE__);
    return;
  }

  let attempts = 0;
  const timer = window.setInterval(() => {
    attempts += 1;
    if (window.__MCP_BRIDGE__) {
      window.clearInterval(timer);
      registerBridgeHelpers(window.__MCP_BRIDGE__);
    } else if (attempts >= 40) {
      window.clearInterval(timer);
    }
  }, 100);
}
