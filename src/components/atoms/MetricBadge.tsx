interface MetricBadgeProps {
  label: string;
  value: string | number | null | undefined;
  unit?: string;
  isBytes?: boolean;
}

export function MetricBadge(props: MetricBadgeProps) {
  const formattedValue = () => {
    if (props.value === null || props.value === undefined) return "—";
    if (props.isBytes && typeof props.value === "number") {
      if (props.value === 0) return "0 MB";
      return `${(props.value / (1024 * 1024)).toFixed(1)} MB`;
    }
    if (typeof props.value === "number" && props.unit === "%") {
      return `${props.value.toFixed(1)}%`;
    }
    return `${props.value}${props.unit ? ` ${props.unit}` : ""}`;
  };

  return (
    <div class="metric-badge">
      <span class="metric-badge-label">{props.label}</span>
      <span class="metric-badge-val font-mono">{formattedValue()}</span>
    </div>
  );
}
