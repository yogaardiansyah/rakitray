import { Channel, invoke } from "@tauri-apps/api/core";
import { createStore, produce } from "solid-js/store";
import type { CatalogItem, InstallProgress, StackTemplate, StoreEvent } from "../types/store";

export interface RakiStoreState {
  catalog: CatalogItem[];
  stackTemplates: StackTemplate[];
  activeProgress: Record<string, InstallProgress>;
  selectedCategory: string;
  isLoading: boolean;
  error: string | null;
}

const [storeState, setStoreState] = createStore<RakiStoreState>({
  catalog: [],
  stackTemplates: [],
  activeProgress: {},
  selectedCategory: "all",
  isLoading: false,
  error: null,
});

export function useRakiStore() {
  async function fetchCatalog(refresh = false) {
    setStoreState("isLoading", true);
    setStoreState("error", null);
    try {
      const items = await invoke<CatalogItem[]>("list_catalog", { refresh });
      const templates = await invoke<StackTemplate[]>("list_stack_templates");
      setStoreState("catalog", items);
      setStoreState("stackTemplates", templates);
    } catch (err) {
      setStoreState("error", String(err));
    } finally {
      setStoreState("isLoading", false);
    }
  }

  async function installRuntime(catalogId: string, onDone?: () => void) {
    const channel = new Channel<StoreEvent>();

    channel.onmessage = (event: StoreEvent) => {
      if (event.type === "progress") {
        setStoreState("activeProgress", event.payload.catalogId, event.payload);
      } else if (event.type === "installedRuntime") {
        setStoreState(
          "catalog",
          produce((items) => {
            const item = items.find((i) => i.entry.id === catalogId);
            if (item) item.installed = true;
          }),
        );
        onDone?.();
      }
    };

    try {
      await invoke("install_runtime", { catalogId, onEvent: channel });
      await fetchCatalog();
    } catch (err) {
      setStoreState(
        "activeProgress",
        catalogId,
        produce((p) => {
          if (p) {
            p.state = "failed";
            p.error = { code: "install_error", detail: String(err) };
          }
        }),
      );
    }
  }

  async function installStack(templateId: string, onDone?: () => void) {
    const channel = new Channel<StoreEvent>();
    channel.onmessage = (event: StoreEvent) => {
      if (event.type === "progress") {
        setStoreState("activeProgress", event.payload.catalogId, event.payload);
      }
    };

    try {
      await invoke("install_stack", { templateId, onEvent: channel });
      await fetchCatalog();
      onDone?.();
    } catch (err) {
      setStoreState("error", String(err));
    }
  }

  async function uninstallRuntime(catalogId: string) {
    try {
      await invoke("uninstall_runtime", { catalogId });
      setStoreState(
        "catalog",
        produce((items) => {
          const item = items.find((i) => i.entry.id === catalogId);
          if (item) {
            item.installed = false;
            item.installPath = null;
          }
        }),
      );
    } catch (err) {
      setStoreState("error", String(err));
    }
  }

  function setCategory(cat: string) {
    setStoreState("selectedCategory", cat);
  }

  return {
    storeState,
    fetchCatalog,
    installRuntime,
    installStack,
    uninstallRuntime,
    setCategory,
  };
}
