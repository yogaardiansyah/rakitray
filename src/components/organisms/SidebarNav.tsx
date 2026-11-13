import { For, Show, createMemo } from "solid-js";
import { useRakiStore } from "../../store/storeState";
import { type ActiveTab, useSupervisor } from "../../store/supervisorStore";
import { Icon, type IconName } from "../atoms";

interface NavItem {
  id: ActiveTab;
  label: string;
  icon: IconName;
  count?: () => string | null;
}

export function SidebarNav() {
  const { state, setActiveTab } = useSupervisor();
  const { storeState } = useRakiStore();

  const runningCount = createMemo(
    () => state.instances.filter((i) => i.status === "running").length,
  );
  const phpCount = createMemo(() => state.instances.filter((i) => i.category === "runtime").length);
  const storeCount = createMemo(() => {
    if (storeState.catalog.length === 0) return null;
    const installed = storeState.catalog.filter((i) => i.installed).length;
    return `${installed}/${storeState.catalog.length}`;
  });

  const sections: { title: string; items: NavItem[] }[] = [
    {
      title: "Workspace",
      items: [
        {
          id: "rack",
          label: "Service Rack",
          icon: "server",
          count: () => `${runningCount()}/${state.instances.length}`,
        },
        {
          id: "sites",
          label: "Sites & Routing",
          icon: "globe",
          count: () => (state.sites.length > 0 ? `${state.sites.length}` : null),
        },
      ],
    },
    {
      title: "Runtimes",
      items: [
        {
          id: "phpMatrix",
          label: "PHP Matrix",
          icon: "sliders",
          count: () => (phpCount() > 0 ? `${phpCount()}` : null),
        },
        {
          id: "runtimes",
          label: "Runtime Binaries",
          icon: "cpu",
          count: () =>
            state.discoveredRuntimes.length > 0 ? `${state.discoveredRuntimes.length}` : null,
        },
        {
          id: "store",
          label: "RakiStore",
          icon: "store",
          count: () => storeCount(),
        },
      ],
    },
  ];

  return (
    <aside class="dense-sidebar-nav" aria-label="Application Sidebar">
      <div class="dense-sidebar-header">
        <div class="logo-box">
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            class="brand-mark"
          >
            <rect x="3" y="4" width="18" height="6" rx="1.5" />
            <rect x="3" y="14" width="18" height="6" rx="1.5" />
            <path d="M7 7h.01M7 17h.01" />
          </svg>
          <span class="logo-text">Rakitray</span>
        </div>
        <span class="version-tag font-mono">0.2</span>
      </div>

      <nav class="dense-sidebar-menu" aria-label="Primary Navigation">
        <For each={sections}>
          {(section) => (
            <div class="sidebar-section">
              <div class="sidebar-section-label">{section.title}</div>
              <For each={section.items}>
                {(item) => {
                  const isActive = () => state.activeTab === item.id;
                  const badgeText = () => item.count?.() ?? null;
                  return (
                    <button
                      type="button"
                      class={`dense-nav-item ${isActive() ? "active" : ""}`}
                      aria-current={isActive() ? "page" : undefined}
                      onClick={() => setActiveTab(item.id)}
                    >
                      <span class="nav-item-left">
                        <Icon name={item.icon} size={15} class="nav-item-icon" />
                        <span class="nav-label">{item.label}</span>
                      </span>
                      <Show when={badgeText()}>
                        {(txt) => <span class="sidebar-count font-mono">{txt()}</span>}
                      </Show>
                    </button>
                  );
                }}
              </For>
            </div>
          )}
        </For>
      </nav>

      <div class="dense-sidebar-footer">
        <div class="status-row">
          <span class="status-label">Supervisor</span>
          <span
            class={`sidebar-supervisor-state font-mono ${state.isSubscribed ? "is-online" : "is-standby"}`}
          >
            <span class="supervisor-dot" aria-hidden="true" />
            {state.isSubscribed ? "Online" : "Standby"}
          </span>
        </div>
        <div class="sidebar-attribution">
          <span>Powered by Rakitray — Created by Yoga ardiansyah</span>
          <a
            href="https://github.com/yogaardiansyah/rakitray"
            target="_blank"
            rel="noreferrer"
            class="attribution-link"
          >
            github.com/yogaardiansyah/rakitray
          </a>
        </div>
      </div>
    </aside>
  );
}
