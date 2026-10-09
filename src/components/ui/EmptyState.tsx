import type { ReactNode } from "react";
import { Dither } from "./Dither";

interface EmptyStateProps {
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({
  title,
  description,
  action,
  className = "",
}: EmptyStateProps) {
  return (
    <div
      className={`flex flex-col items-center justify-center p-8 text-center border border-dashed border-[var(--color-ink-mute)] bg-[var(--color-paper-2)]/30 ${className}`}
    >
      <Dither type="checker" width="64px" height="24px" className="mb-4" />
      <h3 className="font-mono font-bold text-sm uppercase tracking-wider text-[var(--color-ink)] mb-1">
        {title}
      </h3>
      {description && (
        <p className="font-sans text-xs text-[var(--color-ink-mute)] max-w-sm mb-4">
          {description}
        </p>
      )}
      {action && <div>{action}</div>}
    </div>
  );
}
