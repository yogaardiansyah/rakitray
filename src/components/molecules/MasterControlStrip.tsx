import { createMemo } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";

export function MasterControlStrip() {
  const { state, startAllServices, stopAllServices, scanRuntimes } = useSupervisor();

  const totalInstances = () => state.instances.length;
  const runningCount = createMemo(
    () => state.instances.filter((i) => i.status === "running").length,
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
        <button type="button" class="strip-btn btn-start-all" onClick={startAllServices}>
          ▶ Start All
        </button>

        <button type="button" class="strip-btn btn-stop-all" onClick={stopAllServices}>
          ■ Stop All
        </button>

        <button
          type="button"
          class="strip-btn btn-scan"
          disabled={state.isScanning}
          onClick={scanRuntimes}
        >
          {state.isScanning ? "Scanning..." : "↻ Scan Runtimes"}
        </button>
      </div>

      <div class="master-telemetry-group">
        <div class="telemetry-pill font-mono">
          <span class="telemetry-lbl">SERVICES:</span>
          <span class="telemetry-val">
            {runningCount()} / {totalInstances()}
          </span>
        </div>
        <div class="telemetry-pill font-mono">
          <span class="telemetry-lbl">STACK CPU:</span>
          <span class="telemetry-val">{totalCpuPercent().toFixed(1)}%</span>
        </div>
        <div class="telemetry-pill font-mono">
          <span class="telemetry-lbl">STACK RAM:</span>
          <span class="telemetry-val">{formatMemory(totalMemoryBytes())}</span>
        </div>
      </div>
    </div>
  );
}
