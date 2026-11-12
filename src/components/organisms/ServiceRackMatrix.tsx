import { For, Show } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { MasterControlStrip, ServiceRow } from "../molecules";

export function ServiceRackMatrix() {
  const { state, scanRuntimes } = useSupervisor();

  return (
    <div class="service-rack-matrix">
      <MasterControlStrip />

      <div class="dense-matrix-container">
        {/* Sticky Table Header */}
        <div class="dense-table-header">
          <div class="col-service">SERVICE / ENGINE</div>
          <div class="col-version">VERSION & PROFILE</div>
          <div class="col-status">STATUS</div>
          <div class="col-network">PID / PORT</div>
          <div class="col-resources">CPU / MEM</div>
          <div class="col-actions">ACTIONS</div>
        </div>

        <Show
          when={state.instances.length > 0}
          fallback={
            <div class="empty-state-dense">
              <p>No services currently configured in Dynamic Rack.</p>
              <button
                type="button"
                class="dense-action-btn btn-primary-solid"
                onClick={scanRuntimes}
              >
                Scan Installed Runtimes
              </button>
            </div>
          }
        >
          <div class="dense-table-body">
            <For each={state.instances}>{(instance) => <ServiceRow instance={instance} />}</For>
          </div>
        </Show>
      </div>
    </div>
  );
}
