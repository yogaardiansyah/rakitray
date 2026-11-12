import type { PhpProfileMode } from "../../types/phpMatrix";

interface ProfileBadgeProps {
  mode: PhpProfileMode;
}

export function ProfileBadge(props: ProfileBadgeProps) {
  const isDev = () => props.mode === "dev";

  return (
    <span class={`profile-badge font-mono ${isDev() ? "profile-badge-dev" : "profile-badge-prod"}`}>
      {isDev() ? "DEV" : "PROD"}
    </span>
  );
}
