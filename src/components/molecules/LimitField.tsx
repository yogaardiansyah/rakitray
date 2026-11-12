import { createMemo } from "solid-js";

interface LimitFieldProps {
  label: string;
  directive: string;
  value: string;
  isNumberOnly?: boolean;
  disabled?: boolean;
  onInput: (val: string) => void;
}

const INI_LIMIT_REGEX = /^(-1|\d+[KMGkmg]?)$/;
const NUMBER_ONLY_REGEX = /^-?\d+$/;

export function LimitField(props: LimitFieldProps) {
  const isValid = createMemo(() => {
    const val = props.value.trim();
    if (props.isNumberOnly) {
      return NUMBER_ONLY_REGEX.test(val);
    }
    return INI_LIMIT_REGEX.test(val);
  });

  return (
    <div class={`limit-field ${!isValid() ? "limit-invalid" : ""}`}>
      <div class="limit-field-header">
        <label for={props.directive} class="limit-label font-mono">
          {props.label}
        </label>
        <span class="limit-directive font-mono">{props.directive}</span>
      </div>
      <input
        id={props.directive}
        type="text"
        class="dense-input font-mono limit-input"
        value={props.value}
        disabled={props.disabled}
        onInput={(e) => props.onInput(e.currentTarget.value)}
        placeholder={props.isNumberOnly ? "300" : "512M"}
      />
      {!isValid() && (
        <span class="limit-error-hint">
          {props.isNumberOnly ? "Must be an integer" : "Format: -1, 256M, 1G"}
        </span>
      )}
    </div>
  );
}
