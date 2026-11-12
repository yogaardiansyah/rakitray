import { Match, Switch } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import {
  DockedLogTerminal,
  RuntimeDiscoveryGrid,
  ServiceRackMatrix,
  SidebarNav,
  SiteRoutingPanel,
} from "../organisms";

export function WorkstationShell() {
  const { state } = useSupervisor();

  return (
    <div class="app-layout select-none">
      <SidebarNav />

      <div class="main-workspace">
        <main class="workspace-content">
          <Switch>
            <Match when={state.activeTab === "rack"}>
              <ServiceRackMatrix />
            </Match>
            <Match when={state.activeTab === "sites"}>
              <SiteRoutingPanel />
            </Match>
            <Match when={state.activeTab === "runtimes"}>
              <RuntimeDiscoveryGrid />
            </Match>
          </Switch>
        </main>

        <DockedLogTerminal />
      </div>
    </div>
  );
}
