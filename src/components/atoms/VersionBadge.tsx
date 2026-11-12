interface VersionBadgeProps {
  version: string;
  profile?: string;
  engine?: string;
}

export function VersionBadge(props: VersionBadgeProps) {
  return (
    <span class="version-badge font-mono">
      {props.engine ? `${props.engine} ` : ""}v{props.version}
      {props.profile ? ` [${props.profile}]` : ""}
    </span>
  );
}
