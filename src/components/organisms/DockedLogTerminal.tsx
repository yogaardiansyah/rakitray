import { For, createEffect, createMemo } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { LogFilterBar } from "../molecules";

export function DockedLogTerminal() {
  const { state } = useSupervisor();
  let logViewportRef: HTMLDivElement | undefined;

  const filteredLogs = createMemo(() => {
    if (state.logFilter === "all") {
      return state.logs;
    }
    return state.logs.filter((l) => l.instanceId === state.logFilter);
  });

  createEffect(() => {
    if (filteredLogs().length && logViewportRef) {
      logViewportRef.scrollTop = logViewportRef.scrollHeight;
    }
  });

  const formatTimestamp = (ts: number) => {
    const d = new Date(ts);
    const h = String(d.getHours()).padStart(2, "0");
    const m = String(d.getMinutes()).padStart(2, "0");
    const s = String(d.getSeconds()).padStart(2, "0");
    const ms = String(d.getMilliseconds()).padStart(3, "0");
    return `${h}:${m}:${s}.${ms}`;
  };

  return (
    <div class="docked-log-terminal">
      <LogFilterBar />

      <div class="log-viewport font-mono" ref={logViewportRef}>
        <For
          each={filteredLogs()}
          fallback={
            <div class="terminal-empty">
              [Supervisor] Ready. Start a service in the rack to stream live stdout/stderr.
            </div>
          }
        >
          {(entry) => (
            <div class={`terminal-line stream-${entry.stream}`}>
              <span class="term-time">{formatTimestamp(entry.timestampMs)}</span>
              <span class="term-source">
                [{entry.instanceId}:{entry.stream}]
              </span>
              <span class="term-msg">{entry.line}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
