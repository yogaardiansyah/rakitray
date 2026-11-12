import { onMount } from "solid-js";
import { WorkstationShell } from "./components/templates";
import { useSupervisor } from "./store/supervisorStore";

export function App() {
  const { initSupervisorChannel, scanRuntimes } = useSupervisor();

  onMount(() => {
    initSupervisorChannel();
    scanRuntimes();
  });

  return <WorkstationShell />;
}
