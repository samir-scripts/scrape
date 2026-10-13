import type { JobStatus } from "../../bindings/JobStatus";
import { STATUS_CONFIG } from "../../lib/constants";

interface TagProps {
  status?: JobStatus;
  label?: string;
  variant?: "solid" | "outline" | "muted";
  className?: string;
}

export function Tag({
  status,
  label,
  variant = "solid",
  className = "",
}: TagProps) {
  if (status && STATUS_CONFIG[status]) {
    const config = STATUS_CONFIG[status];
    const isDither = config.dither === "checker";

    return (
      <span
        style={{
          backgroundColor: config.bg,
          color: config.text,
          borderColor: config.border,
        }}
        className={`inline-flex items-center px-1.5 py-0.5 text-xs font-mono uppercase tracking-wider border leading-none select-none ${
          isDither ? "dither-checker" : ""
        } ${className}`}
      >
        {config.label}
      </span>
    );
  }

  const baseStyle =
    variant === "outline"
      ? "border border-[var(--color-ink)] bg-transparent text-[var(--color-ink)]"
      : variant === "muted"
      ? "border border-[var(--color-ink-mute)] bg-[var(--color-paper-2)] text-[var(--color-ink-mute)]"
      : "border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)]";

  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 text-xs font-mono uppercase tracking-wider leading-none select-none ${baseStyle} ${className}`}
    >
      {label}
    </span>
  );
}
