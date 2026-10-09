import type { HTMLAttributes } from "react";

interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  variant?: "paper" | "paper-2" | "inverted";
  bordered?: boolean;
}

export function Panel({
  variant = "paper",
  bordered = true,
  className = "",
  children,
  ...props
}: PanelProps) {
  const bg =
    variant === "paper-2"
      ? "bg-[var(--color-paper-2)] text-[var(--color-ink)]"
      : variant === "inverted"
      ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
      : "bg-[var(--color-paper)] text-[var(--color-ink)]";

  const border = bordered ? "border border-[var(--color-ink)]" : "";

  return (
    <div className={`${bg} ${border} ${className}`} {...props}>
      {children}
    </div>
  );
}
