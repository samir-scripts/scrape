import { Icon } from "./Icon";
import type { HTMLAttributes } from "react";

interface ListRowProps extends HTMLAttributes<HTMLDivElement> {
  selected?: boolean;
  disabled?: boolean;
}

export function ListRow({
  selected = false,
  disabled = false,
  className = "",
  children,
  ...props
}: ListRowProps) {
  const selectedClass = selected
    ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold shadow-none"
    : "bg-transparent text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]";

  const disabledClass = disabled
    ? "opacity-40 cursor-not-allowed pointer-events-none"
    : "cursor-pointer";

  return (
    <div
      role="row"
      tabIndex={0}
      className={`relative px-3 py-2 border-b border-[var(--color-ink)] flex items-center justify-between text-xs select-none transition-none ${selectedClass} ${disabledClass} ${className}`}
      {...props}
    >
      {selected && (
        <span className="absolute left-1 text-[var(--color-paper)] font-mono text-xs select-none">
          <Icon name="right" size="sm" />
        </span>
      )}
      <div className={`w-full ${selected ? "pl-2" : ""}`}>{children}</div>
    </div>
  );
}
