import { createMemo } from "solid-js";
import { type ActiveTab, useSupervisor } from "../../store/supervisorStore";
import { StatusLed } from "../atoms";

export function SidebarNav() {
  const { state, setActiveTab } = useSupervisor();

  const runningCount = createMemo(
    () => state.instances.filter((i) => i.status === "running").length,
  );
  const totalCount = () => state.instances.length;
  const discoveredCount = () => state.discoveredRuntimes.length;

  const navItems: { id: ActiveTab; label: string; badge: () => string; pillClass?: string }[] = [
    {
      id: "rack",
      label: "Service Rack",
      badge: () => `${runningCount()}/${totalCount()}`,
      pillClass: "badge-running",
    },
    {
      id: "sites",
      label: "Sites & Routing",
      badge: () => "Laravel",
      pillClass: "badge-laravel",
    },
    {
      id: "runtimes",
      label: "Runtime Binaries",
      badge: () => `${discoveredCount()}`,
      pillClass: "badge-runtimes",
    },
  ];

  return (
    <aside class="dense-sidebar-nav">
      <div class="dense-sidebar-header">
        <div class="logo-box">
          <StatusLed status="running" size="sm" />
          <span class="logo-text">RAKITRAY</span>
        </div>
        <span class="version-tag">v0.1.0</span>
      </div>

      <nav class="dense-sidebar-menu">
        {navItems.map((item) => (
          <button
            type="button"
            class={`dense-nav-item ${state.activeTab === item.id ? "active" : ""}`}
            onClick={() => setActiveTab(item.id)}
          >
            <span class="nav-label">{item.label}</span>
            <span class={`dense-nav-badge ${item.pillClass ?? ""}`}>{item.badge()}</span>
          </button>
        ))}
      </nav>

      <div class="dense-sidebar-footer font-mono">
        <div class="status-row">
          <span class="status-label">SUPERVISOR</span>
          <span class="status-val text-emerald">ACTIVE</span>
        </div>
        <div class="status-row">
          <span class="status-label">ANTI-ZOMBIE</span>
          <span class="status-val text-sky">PR_SET_PDEATHSIG</span>
        </div>
        <div class="status-row">
          <span class="status-label">IPC STREAM</span>
          <span class="status-val text-slate">60Hz Channel</span>
        </div>
      </div>
    </aside>
  );
}
