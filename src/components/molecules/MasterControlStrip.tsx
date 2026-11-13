import { createMemo } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";

export function MasterControlStrip() {
  const { state, startAllServices, stopAllServices } = useSupervisor();

  const totalInstances = () => state.instances.length;
  const runningCount = createMemo(
    () => state.instances.filter((i) => i.status === "running").length,
  );
  const stoppedCount = createMemo(
    () => state.instances.filter((i) => i.status === "stopped").length,
  );

  const totalCpuPercent = createMemo(() =>
    state.instances.reduce((acc, curr) => acc + (curr.cpuPercent || 0), 0),
  );

  const totalMemoryBytes = createMemo(() =>
    state.instances.reduce((acc, curr) => acc + (curr.memoryBytes || 0), 0),
  );

  const formatMemory = (bytes: number) => {
    if (!bytes) return "0 MB";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div class="dense-master-strip">
      <div class="master-btn-group">
        <button
          type="button"
          class="strip-btn btn-start-all"
          disabled={stoppedCount() === 0}
          onClick={startAllServices}
        >
          <Icon name="play" size={12} />
          <span>Start All</span>
        </button>

        <button
          type="button"
          class="strip-btn btn-stop-all"
          disabled={runningCount() === 0}
          onClick={stopAllServices}
        >
          <Icon name="stop" size={12} />
          <span>Stop All</span>
        </button>
      </div>

      <div class="master-telemetry-summary font-mono">
        <span>
          <strong class="telemetry-val">
            {runningCount()}/{totalInstances()}
          </strong>{" "}
          running
        </span>
        <span class="topbar-dot">·</span>
        <span>
          <strong class="telemetry-val">{totalCpuPercent().toFixed(1)}%</strong> CPU
        </span>
        <span class="topbar-dot">·</span>
        <span>
          <strong class="telemetry-val">{formatMemory(totalMemoryBytes())}</strong> RAM
        </span>
      </div>
    </div>
  );
}
