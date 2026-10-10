import { useMemo } from "react";
import type { SalaryDistributionBucket } from "../../../bindings/SalaryDistributionBucket";

interface SalaryDistributionBarProps {
  distribution: SalaryDistributionBucket[];
  medianSalary: number | null;
  avgSalary: number | null;
  className?: string;
}

export function SalaryDistributionBar({
  distribution,
  medianSalary,
  avgSalary,
  className = "",
}: SalaryDistributionBarProps) {
  const total = useMemo(
    () => distribution.reduce((sum, b) => sum + b.count, 0),
    [distribution]
  );

  const maxBucketCount = useMemo(
    () => Math.max(...distribution.map((b) => b.count), 1),
    [distribution]
  );

  return (
    <div
      className={`border border-[var(--color-ink)] bg-[var(--color-paper)] p-4 flex flex-col justify-between select-none ${className}`}
    >
      <div className="flex items-center justify-between border-b border-[var(--color-ink-mute)]/30 pb-3">
        <span className="font-mono text-xs font-bold uppercase tracking-wider text-[var(--color-ink)]">
          SALARY DISTRIBUTION
        </span>
        <div className="flex items-center gap-3 font-mono text-[10px] text-[var(--color-ink-mute)]">
          {medianSalary && (
            <span>MEDIAN: ${Math.round(medianSalary).toLocaleString()}</span>
          )}
          {avgSalary && (
            <span>AVG: ${Math.round(avgSalary).toLocaleString()}</span>
          )}
        </div>
      </div>

      <div className="py-3 flex flex-col gap-2">
        {distribution.map((bucket) => {
          const pctOfTotal = total > 0 ? Math.round((bucket.count / total) * 100) : 0;
          const barWidthPct = total > 0 ? Math.round((bucket.count / maxBucketCount) * 100) : 0;

          return (
            <div key={bucket.range_label} className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between font-mono text-[11px]">
                <span className="text-[var(--color-ink)] font-bold">
                  {bucket.range_label}
                </span>
                <span className="text-[var(--color-ink-mute)] tabular-nums">
                  {bucket.count} ({pctOfTotal}%)
                </span>
              </div>
              <div className="w-full h-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 p-[1px]">
                {barWidthPct > 0 && (
                  <div
                    style={{ width: `${barWidthPct}%` }}
                    className="h-full bg-[var(--color-ink)] dither-checker"
                  />
                )}
              </div>
            </div>
          );
        })}
      </div>

      <div className="border-t border-[var(--color-ink)] pt-2 mt-2 flex items-center justify-between font-mono text-[11px]">
        <span className="font-black uppercase text-[var(--color-ink)]">
          POSTINGS WITH SALARY:
        </span>
        <span className="font-bold text-[var(--color-ink)] tabular-nums">
          {total} INGESTED
        </span>
      </div>
    </div>
  );
}
