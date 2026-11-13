export type IconName =
  | "server"
  | "globe"
  | "sliders"
  | "cpu"
  | "store"
  | "play"
  | "stop"
  | "refresh"
  | "terminal"
  | "search"
  | "copy"
  | "check"
  | "x"
  | "alert"
  | "chevronDown"
  | "chevronUp"
  | "zap"
  | "plus"
  | "externalLink"
  | "edit";

interface IconProps {
  name: IconName;
  size?: number;
  class?: string;
}

const PATHS: Record<IconName, string> = {
  server:
    "M2 5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5Zm0 10a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-4ZM6 7h.01M6 17h.01",
  globe:
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 0c2.5 2.7 4 6.2 4 10s-1.5 7.3-4 10m0-20c-2.5 2.7-4 6.2-4 10s1.5 7.3 4 10M2 12h20",
  sliders: "M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6",
  cpu: "M9 2v2M15 2v2M9 20v2M15 20v2M2 9h2M2 15h2M20 9h2M20 15h2M6 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm3 5h6v6H9V9Z",
  store: "M3 9l1-5h16l1 5M3 9v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9M3 9h18M9 21V13h6v8",
  play: "M6 4l14 8-14 8V4Z",
  stop: "M6 6h12v12H6z",
  refresh: "M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6",
  terminal: "M4 17l6-6-6-6M12 19h8",
  search: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16ZM21 21l-4.35-4.35",
  copy: "M9 9h11v11H9zM5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1",
  check: "M20 6L9 17l-5-5",
  x: "M18 6L6 18M6 6l12 12",
  alert:
    "M12 9v4M12 17h.01M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z",
  chevronDown: "M6 9l6 6 6-6",
  chevronUp: "M18 15l-6-6-6 6",
  zap: "M13 2L3 14h9l-1 8 10-12h-9l1-8Z",
  plus: "M12 5v14M5 12h14",
  externalLink: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6M15 3h6v6M10 14L21 3",
  edit: "M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3Z",
};

export function Icon(props: IconProps) {
  const size = () => props.size ?? 14;
  return (
    <svg
      width={size()}
      height={size()}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
      class={`ui-icon ${props.class ?? ""}`}
    >
      <path d={PATHS[props.name]} />
    </svg>
  );
}
