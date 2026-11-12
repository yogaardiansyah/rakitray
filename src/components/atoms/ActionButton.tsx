import type { JSX } from "solid-js";

interface ActionButtonProps {
  children: JSX.Element;
  onClick?: () => void;
  variant?: "primary" | "danger" | "ghost" | "icon";
  disabled?: boolean;
  loading?: boolean;
  type?: "button" | "submit" | "reset";
  title?: string;
}

export function ActionButton(props: ActionButtonProps) {
  const variantClass = () => {
    switch (props.variant) {
      case "danger":
        return "btn-atom-danger";
      case "ghost":
        return "btn-atom-ghost";
      case "icon":
        return "btn-atom-icon";
      default:
        return "btn-atom-primary";
    }
  };

  return (
    <button
      type={props.type ?? "button"}
      class={`btn-atom ${variantClass()} ${props.loading ? "btn-loading" : ""}`}
      disabled={props.disabled || props.loading}
      onClick={props.onClick}
      title={props.title}
    >
      {props.children}
    </button>
  );
}
