import type { ReactNode } from "react";

interface FieldProps {
  label?: string;
  hint?: string;
  error?: string;
  className?: string;
  children: ReactNode;
}

export function Field({
  label,
  hint,
  error,
  className = "",
  children,
}: FieldProps) {
  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      {label && (
        <div className="flex justify-between items-center text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
          <span>{label}</span>
          {hint && <span className="text-[10px] text-[var(--color-ink-mute)] font-normal">{hint}</span>}
        </div>
      )}
      {children}
      {error && (
        <span className="text-[11px] font-mono text-[var(--color-cherry)]">
          {error}
        </span>
      )}
    </div>
  );
}
