import { ActionButton } from "../atoms";

interface PendingBarProps {
  hasPending: boolean;
  isApplying: boolean;
  isValid: boolean;
  onApply: () => void;
  onDiscard: () => void;
}

export function PendingBar(props: PendingBarProps) {
  if (!props.hasPending) return null;

  return (
    <div class="pending-bar">
      <div class="pending-info">
        <span class="pending-dot" />
        <span class="pending-text">
          Unsaved profile / limit changes staged. Apply will validate and reload PHP.
        </span>
      </div>
      <div class="pending-actions">
        <ActionButton variant="ghost" disabled={props.isApplying} onClick={props.onDiscard}>
          Discard
        </ActionButton>
        <ActionButton
          variant="primary"
          disabled={!props.isValid || props.isApplying}
          loading={props.isApplying}
          onClick={props.onApply}
        >
          Apply & Reload PHP
        </ActionButton>
      </div>
    </div>
  );
}
