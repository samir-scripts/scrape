import { Dither } from "../../ui/Dither";

interface MetricCardProps {
  label: string;
  value: string | number;
  subline: string;
  ditherType?: "dots" | "checker" | "lines";
  className?: string;
}

export function MetricCard({
  label,
  value,
  subline,
  ditherType = "dots",
  className = "",
}: MetricCardProps) {
  return (
    <div
      className={`border border-[var(--color-ink)] bg-[var(--color-paper)] p-4 flex flex-col justify-between relative select-none ${className}`}
    >
      <div className="flex items-start justify-between">
        <span className="font-mono text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
          {label}
        </span>
        <Dither type={ditherType} width="32px" height="12px" />
      </div>

      <div className="my-2">
        <span className="text-3xl font-black font-mono tracking-tight text-[var(--color-ink)] tabular-nums">
          {value}
        </span>
      </div>

      <div className="border-t border-[var(--color-ink-mute)]/30 pt-1.5 mt-auto">
        <span className="font-mono text-xs text-[var(--color-ink-mute)] uppercase tracking-wider truncate block">
          {subline}
        </span>
      </div>
    </div>
  );
}
