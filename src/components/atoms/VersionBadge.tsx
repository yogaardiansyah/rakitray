import { Show } from "solid-js";

interface VersionBadgeProps {
  version: string;
  profile?: string;
  engine?: string;
}

export function VersionBadge(props: VersionBadgeProps) {
  return (
    <span class="version-badge-group font-mono">
      <span class="version-number">{props.version}</span>
      <Show when={props.profile}>{(p) => <span class="version-profile-tag">{p()}</span>}</Show>
    </span>
  );
}
