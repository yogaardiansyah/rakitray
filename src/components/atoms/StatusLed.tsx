import type { ServiceStatus } from "../../types/ipc";

interface StatusLedProps {
  status: ServiceStatus;
  size?: "sm" | "md" | "lg";
}

export function StatusLed(props: StatusLedProps) {
  const sizeClass = () => {
    switch (props.size) {
      case "sm":
        return "led-sm";
      case "lg":
        return "led-lg";
      default:
        return "led-md";
    }
  };

  const statusClass = () => `led-${props.status}`;

  return (
    <span
      class={`status-led ${sizeClass()} ${statusClass()}`}
      title={`Status: ${props.status}`}
      aria-label={`Status: ${props.status}`}
    />
  );
}
