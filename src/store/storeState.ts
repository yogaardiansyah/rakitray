import { Channel, invoke } from "@tauri-apps/api/core";
import { createStore, produce } from "solid-js/store";
import type { CatalogItem, InstallProgress, StackTemplate, StoreEvent } from "../types/store";
import { useSupervisor } from "./supervisorStore";

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
    const { appendLocalLog } = useSupervisor();
    const channel = new Channel<StoreEvent>();
    let lastState = "";

    appendLocalLog("store", "system", `Starting install: ${catalogId}`);
    channel.onmessage = (event: StoreEvent) => {
      if (event.type === "progress") {
        setStoreState("activeProgress", event.payload.catalogId, event.payload);
        if (event.payload.state !== lastState) {
          lastState = event.payload.state;
          appendLocalLog(
            "store",
            event.payload.state === "failed" ? "stderr" : "system",
            `[${catalogId}] ${event.payload.state}${event.payload.error?.detail ? `: ${event.payload.error.detail}` : event.payload.message ? ` — ${event.payload.message}` : ""}`,
          );
        }
      } else if (event.type === "installedRuntime") {
        setStoreState(
          "catalog",
          produce((items) => {
            const item = items.find((i) => i.entry.id === catalogId);
            if (item) item.installed = true;
          }),
        );
        appendLocalLog("store", "stdout", `Installed ${catalogId}`);
        onDone?.();
      }
    };

    try {
      await invoke("install_runtime", { catalogId, onEvent: channel });
      await fetchCatalog();
    } catch (err) {
      const detail = String(err);
      appendLocalLog("store", "stderr", `[${catalogId}] Install failed: ${detail}`);
      setStoreState(
        "activeProgress",
        catalogId,
        produce((p) => {
          if (p) {
            p.state = "failed";
            p.error = { code: "install_error", detail };
          }
        }),
      );
    }
  }

  async function installStack(templateId: string, onDone?: () => void) {
    const { appendLocalLog } = useSupervisor();
    const channel = new Channel<StoreEvent>();
    appendLocalLog("store", "system", `Installing stack: ${templateId}`);
    channel.onmessage = (event: StoreEvent) => {
      if (event.type === "progress") {
        setStoreState("activeProgress", event.payload.catalogId, event.payload);
      }
    };

    try {
      await invoke("install_stack", { templateId, onEvent: channel });
      appendLocalLog("store", "stdout", `Stack installed: ${templateId}`);
      await fetchCatalog();
      onDone?.();
    } catch (err) {
      const msg = String(err);
      appendLocalLog("store", "stderr", `Stack ${templateId} failed: ${msg}`);
      setStoreState("error", msg);
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

  async function cancelInstall(catalogId: string) {
    const installId = storeState.activeProgress[catalogId]?.installId || catalogId;
    try {
      await invoke("cancel_install", { installId });
      setStoreState(
        "activeProgress",
        catalogId,
        produce((p) => {
          if (p) p.state = "cancelled";
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
    cancelInstall,
    installStack,
    uninstallRuntime,
    setCategory,
  };
}
