import type { PhpProfileMode } from "../../types/phpMatrix";

interface ProfileToggleProps {
  mode: PhpProfileMode;
  disabled?: boolean;
  onChange: (mode: PhpProfileMode) => void;
}

export function ProfileToggle(props: ProfileToggleProps) {
  return (
    <div class="profile-toggle-container">
      <span class="profile-toggle-label">PROFILE:</span>
      <div class="profile-toggle-group">
        <button
          type="button"
          class={`profile-toggle-btn ${props.mode === "dev" ? "active-dev" : ""}`}
          disabled={props.disabled}
          onClick={() => props.onChange("dev")}
          title="Development Profile (display_errors=On, full logging, higher limits)"
        >
          DEV
        </button>
        <button
          type="button"
          class={`profile-toggle-btn ${props.mode === "prod" ? "active-prod" : ""}`}
          disabled={props.disabled}
          onClick={() => props.onChange("prod")}
          title="Production Profile (display_errors=Off, opcache optimized, conservative limits)"
        >
          PROD
        </button>
      </div>
    </div>
  );
}
