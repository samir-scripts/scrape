import type { ButtonHTMLAttributes } from "react";
import { KeyHint } from "./KeyHint";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "inverted" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  shortcut?: string;
}

export function Button({
  variant = "default",
  size = "md",
  shortcut,
  className = "",
  disabled = false,
  children,
  ...props
}: ButtonProps) {
  const sizeClass =
    size === "sm"
      ? "px-2 py-1 text-xs"
      : size === "lg"
      ? "px-4 py-2.5 text-sm"
      : "px-3 py-1.5 text-xs";

  let variantClass =
    "border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] active:bg-[var(--color-ink)] active:text-[var(--color-paper)]";

  if (variant === "inverted") {
    variantClass =
      "border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:bg-[var(--color-paper)] hover:text-[var(--color-ink)]";
  } else if (variant === "ghost") {
    variantClass =
      "border border-transparent hover:border-[var(--color-ink)] text-[var(--color-ink)] bg-transparent hover:bg-[var(--color-paper-2)]";
  } else if (variant === "danger") {
    variantClass =
      "border border-[var(--color-danger)] text-[var(--color-danger)] bg-transparent hover:bg-[var(--color-danger)] hover:text-[var(--color-paper)]";
  }

  const disabledClass = disabled
    ? "opacity-40 cursor-not-allowed pointer-events-none border-[var(--color-ink-mute)] text-[var(--color-ink-mute)]"
    : "cursor-pointer";

  return (
    <button
      disabled={disabled}
      className={`inline-flex items-center justify-center gap-2 font-mono font-bold uppercase tracking-wider transition-colors select-none ${sizeClass} ${variantClass} ${disabledClass} ${className}`}
      {...props}
    >
      <span>{children}</span>
      {shortcut && (
        <KeyHint
          keys={shortcut}
          className="ml-1 opacity-70 group-hover:opacity-100"
        />
      )}
    </button>
  );
}
