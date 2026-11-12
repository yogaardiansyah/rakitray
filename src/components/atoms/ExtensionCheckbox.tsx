import type { PhpExtension } from "../../types/phpMatrix";

interface ExtensionCheckboxProps {
  extension: PhpExtension;
  disabled?: boolean;
  onToggle: (name: string, enabled: boolean) => void;
}

export function ExtensionCheckbox(props: ExtensionCheckboxProps) {
  const isLocked = () =>
    props.disabled || !props.extension.toggleable || props.extension.source === "builtIn";

  const tooltipText = () => {
    if (props.extension.source === "builtIn") {
      return `PHP built-in extension (${props.extension.name}) — compiled into binary, cannot be disabled`;
    }
    if (!props.extension.toggleable) {
      return `Locked extension (${props.extension.name})`;
    }
    if (props.extension.requires.length > 0) {
      return `Requires: ${props.extension.requires.join(", ")}`;
    }
    return `Toggle ${props.extension.name}`;
  };

  return (
    <label
      class={`ext-checkbox-card ${props.extension.enabled ? "ext-active" : "ext-inactive"} ${isLocked() ? "ext-locked" : ""}`}
      title={tooltipText()}
    >
      <input
        type="checkbox"
        class="ext-input"
        checked={props.extension.enabled}
        disabled={isLocked()}
        onChange={(e) => props.onToggle(props.extension.name, e.currentTarget.checked)}
      />
      <div class="ext-details">
        <div class="ext-header">
          <span class="ext-name font-mono">{props.extension.name}</span>
          <div class="ext-tags">
            {props.extension.kind === "zendExtension" && <span class="ext-tag tag-zend">zend</span>}
            {props.extension.source === "builtIn" && (
              <span class="ext-tag tag-builtin">built-in</span>
            )}
          </div>
        </div>
        {props.extension.requires.length > 0 && (
          <span class="ext-requires font-mono">req: {props.extension.requires.join(", ")}</span>
        )}
      </div>
    </label>
  );
}
