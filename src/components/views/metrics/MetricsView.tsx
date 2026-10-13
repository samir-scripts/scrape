import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../../lib/api";
import { Dither } from "../../ui/Dither";
import { EmptyState } from "../../ui/EmptyState";
import { MetricCard } from "./MetricCard";
import { ApplicationsChart } from "./ApplicationsChart";
import { RoleTypeDonut } from "./RoleTypeDonut";
import { SalaryDistributionBar } from "./SalaryDistributionBar";

export function MetricsView() {
  const [granularity, setGranularity] = useState<"week" | "month">("week");
  const [periods, setPeriods] = useState<number>(12);

  const { data: metrics, isLoading, error } = useQuery({
    queryKey: ["metrics", granularity, periods],
    queryFn: () => api.getMetrics(granularity, periods),
  });

  if (isLoading && !metrics) {
    return (
      <div className="flex-1 w-full h-full p-6 bg-[var(--color-paper)] flex flex-col items-center justify-center gap-3 select-none">
        <Dither type="checker" width="96px" height="24px" />
        <span className="font-mono text-xs uppercase tracking-wider text-[var(--color-ink-mute)]">
          LOADING METRICS...
        </span>
      </div>
    );
  }

  if (error || !metrics) {
    return (
      <div className="flex-1 w-full h-full p-6 bg-[var(--color-paper)] flex flex-col items-center justify-center gap-3 select-none">
        <span className="font-mono text-xs uppercase tracking-wider text-[var(--color-cherry)]">
          FAILED TO LOAD METRICS
        </span>
      </div>
    );
  }

  const hasData = metrics.totals.scraped > 0 || metrics.totals.applied > 0;

  return (
    <div className="flex-1 w-full h-full overflow-y-auto p-6 bg-[var(--color-paper)] flex flex-col gap-6 select-none">
      {/* Header Bar */}
      <div className="flex items-center justify-between border-b border-[var(--color-ink)] pb-3">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-black uppercase tracking-tight text-[var(--color-ink)] font-mono">
            METRICS
          </h1>
          {metrics.streak.current > 0 && (
            <span className="border border-[var(--color-ink)] px-2 py-0.5 font-mono text-xs font-bold uppercase bg-[var(--color-paper-2)]">
              ACTIVE STREAK: {metrics.streak.current} {metrics.streak.current === 1 ? "DAY" : "DAYS"}
            </span>
          )}
        </div>
        <Dither type="checker" width="96px" height="20px" />
      </div>

      {!hasData ? (
        <div className="flex-1 flex items-center justify-center">
          <EmptyState
            title="NO APPLICATIONS LOGGED. PRESS [A] ON A JOB TO BEGIN."
            className="w-full max-w-md"
          />
        </div>
      ) : (
        <>
          {/* KPI Cards Row */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <MetricCard
              label="TOTAL SCRAPED"
              value={metrics.totals.scraped.toLocaleString()}
              subline="ALL-TIME POSTINGS INGESTED"
              ditherType="checker"
            />

            <MetricCard
              label="TOTAL APPLIED"
              value={metrics.totals.applied.toLocaleString()}
              subline={
                metrics.streak.current > 0
                  ? `STREAK: ${metrics.streak.current}D · BEST: ${metrics.streak.longest}D`
                  : `LONGEST STREAK: ${metrics.streak.longest} DAYS`
              }
              ditherType="dots"
            />

            <MetricCard
              label="RESPONSE RATE"
              value={`${Math.round(metrics.totals.response_rate * 100)}%`}
              subline={`${metrics.totals.interviewing + metrics.totals.offers} / ${metrics.totals.applied} CONVERTED`}
              ditherType="lines"
            />

            <MetricCard
              label="MEDIAN SALARY"
              value={
                metrics.totals.median_salary
                  ? `$${Math.round(metrics.totals.median_salary).toLocaleString()}`
                  : "--"
              }
              subline={
                metrics.totals.avg_salary
                  ? `AVG: $${Math.round(metrics.totals.avg_salary).toLocaleString()}`
                  : "ANNUAL BASE COMPENSATION"
              }
              ditherType="dots"
            />
          </div>

          {/* Primary Visualization Row (2fr 1fr grid) */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-[380px]">
            <div className="lg:col-span-2 h-full">
              <ApplicationsChart
                series={metrics.series}
                granularity={granularity}
                periods={periods}
                onGranularityChange={setGranularity}
                onPeriodsChange={setPeriods}
              />
            </div>

            <div className="lg:col-span-1 h-full">
              <RoleTypeDonut data={metrics.by_role_type} />
            </div>
          </div>

          {/* Secondary Salary Distribution Section */}
          {metrics.salary_distribution.some((b) => b.count > 0) && (
            <div className="w-full">
              <SalaryDistributionBar
                distribution={metrics.salary_distribution}
                medianSalary={metrics.totals.median_salary}
                avgSalary={metrics.totals.avg_salary}
              />
            </div>
          )}
        </>
      )}
    </div>
  );
}
