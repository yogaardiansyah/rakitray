import { For, createEffect } from "solid-js";
import type { LogEntry } from "../types/ipc";

interface LiveLogPanelProps {
  logs: LogEntry[];
  onClear: () => void;
}

export function LiveLogPanel(props: LiveLogPanelProps) {
  let logContainerRef: HTMLDivElement | undefined;

  createEffect(() => {
    if (props.logs.length && logContainerRef) {
      logContainerRef.scrollTop = logContainerRef.scrollHeight;
    }
  });

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    return d.toTimeString().split(" ")[0];
  };

  return (
    <div class="card log-panel">
      <div class="log-header">
        <h3>Live Log Channel (16ms Micro-Batch)</h3>
        <button type="button" class="btn btn-secondary" onClick={props.onClear}>
          Clear Logs
        </button>
      </div>

      <div class="log-stream" ref={logContainerRef}>
        <For
          each={props.logs}
          fallback={
            <div class="log-empty">No log messages yet. Start a service to stream output.</div>
          }
        >
          {(entry) => (
            <div class={`log-line log-${entry.stream}`}>
              <span class="log-time">[{formatTime(entry.timestampMs)}]</span>
              <span class="log-id">[{entry.instanceId}]</span>
              <span class="log-text">{entry.line}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
