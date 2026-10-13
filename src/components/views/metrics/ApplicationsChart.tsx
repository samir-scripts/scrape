import { useState, useMemo, useRef, useEffect } from "react";
import { scaleBand, scaleLinear } from "@visx/scale";
import { Group } from "@visx/group";
import { Bar } from "@visx/shape";
import { ParentSize } from "@visx/responsive";
import type { MetricsSeriesBucket } from "../../../bindings/MetricsSeriesBucket";

interface ApplicationsChartProps {
  series: MetricsSeriesBucket[];
  granularity: "week" | "month";
  periods: number;
  onGranularityChange: (g: "week" | "month") => void;
  onPeriodsChange: (p: number) => void;
}

function formatReadout(bucket: MetricsSeriesBucket, granularity: "week" | "month"): string {
  if (granularity === "week") {
    try {
      const start = new Date(bucket.bucket_start + "T00:00:00Z");
      const end = new Date(start.getTime() + 6 * 86400000);
      const startStr = bucket.bucket_start;
      const endMonth = String(end.getUTCMonth() + 1).padStart(2, "0");
      const endDay = String(end.getUTCDate()).padStart(2, "0");
      return `WEEK ${bucket.label.replace(/^W/, "")} · ${startStr} → ${endMonth}-${endDay} · ${bucket.count} APPLIED`;
    } catch {
      return `${bucket.label} · ${bucket.bucket_start} · ${bucket.count} APPLIED`;
    }
  } else {
    return `${bucket.label} · ${bucket.bucket_start.slice(0, 7)} · ${bucket.count} APPLIED`;
  }
}

interface InnerChartProps {
  width: number;
  height: number;
  series: MetricsSeriesBucket[];
  granularity: "week" | "month";
  selectedIndex: number | null;
  onSelectIndex: (idx: number | null) => void;
}

function InnerChart({
  width,
  height,
  series,
  granularity: _granularity,
  selectedIndex,
  onSelectIndex,
}: InnerChartProps) {
  const margin = { top: 16, right: 16, bottom: 28, left: 32 };
  const xMax = Math.max(0, width - margin.left - margin.right);
  const yMax = Math.max(0, height - margin.top - margin.bottom);

  const getX = (d: MetricsSeriesBucket) => d.bucket_start;
  const getY = (d: MetricsSeriesBucket) => d.count;

  const xScale = useMemo(
    () =>
      scaleBand<string>({
        range: [0, xMax],
        round: true,
        domain: series.map(getX),
        padding: 0.28,
      }),
    [xMax, series]
  );

  const maxVal = Math.max(...series.map(getY), 1);
  const yScale = useMemo(
    () =>
      scaleLinear<number>({
        range: [yMax, 0],
        round: true,
        domain: [0, Math.ceil(maxVal * 1.15)],
      }),
    [yMax, maxVal]
  );

  const yTicks = useMemo(() => {
    const ticks = yScale.ticks(4);
    return Array.from(new Set([0, ...ticks.map((t) => Math.round(t))])).sort((a, b) => a - b);
  }, [yScale]);

  return (
    <svg
      width={width}
      height={height}
      style={{ overflow: "visible" }}
      shapeRendering="crispEdges"
    >
      <defs>
        {/* Crisp monochrome halftone dot pattern */}
        <pattern
          id="chart-halftone-pattern"
          width="4"
          height="4"
          patternUnits="userSpaceOnUse"
        >
          <rect width="4" height="4" fill="var(--color-paper)" />
          <circle cx="2" cy="2" r="1" fill="var(--color-ink)" />
        </pattern>
      </defs>

      <Group left={margin.left} top={margin.top}>
        {/* Horizontal gridlines */}
        {yTicks.map((tickVal) => {
          const y = Math.round(yScale(tickVal));
          return (
            <g key={`ytick-${tickVal}`}>
              <line
                x1={0}
                x2={xMax}
                y1={y}
                y2={y}
                stroke={tickVal === 0 ? "var(--color-ink)" : "var(--color-ink-mute)"}
                strokeWidth={1}
                strokeDasharray={tickVal === 0 ? undefined : "2 2"}
                opacity={tickVal === 0 ? 1 : 0.4}
              />
              <text
                x={-6}
                y={y + 3}
                textAnchor="end"
                className="fill-[var(--color-ink-mute)] font-mono text-[9px] select-none"
              >
                {tickVal}
              </text>
            </g>
          );
        })}

        {/* Bars */}
        {series.map((d, idx) => {
          const x = Math.round(xScale(getX(d)) ?? 0);
          const barWidth = Math.max(1, Math.round(xScale.bandwidth()));
          const count = getY(d);
          const barY = Math.round(yScale(count));
          const barHeight = Math.max(0, yMax - barY);
          const isSelected = selectedIndex === idx;

          return (
            <g
              key={`bar-${d.bucket_start}`}
              onMouseEnter={() => onSelectIndex(idx)}
              className="cursor-pointer"
            >
              {count > 0 ? (
                <Bar
                  x={x}
                  y={barY}
                  width={barWidth}
                  height={barHeight}
                  fill={isSelected ? "var(--color-ink)" : "url(#chart-halftone-pattern)"}
                  stroke="var(--color-ink)"
                  strokeWidth={1}
                />
              ) : (
                /* 0 count indicator */
                <rect
                  x={x}
                  y={yMax - 2}
                  width={barWidth}
                  height={2}
                  fill={isSelected ? "var(--color-ink)" : "var(--color-ink-mute)"}
                  opacity={isSelected ? 1 : 0.3}
                />
              )}

              {/* Invisible touch/hover hit area */}
              <rect
                x={x}
                y={0}
                width={barWidth}
                height={yMax}
                fill="transparent"
              />

              {/* X Axis Label */}
              <text
                x={x + barWidth / 2}
                y={yMax + 16}
                textAnchor="middle"
                className={`font-mono text-xs select-none ${
                  isSelected
                    ? "fill-[var(--color-ink)] font-black"
                    : "fill-[var(--color-ink-mute)]"
                }`}
              >
                {d.label}
              </text>
            </g>
          );
        })}

        {/* X Axis Baseline */}
        <line
          x1={0}
          x2={xMax}
          y1={yMax}
          y2={yMax}
          stroke="var(--color-ink)"
          strokeWidth={1}
        />
      </Group>
    </svg>
  );
}

export function ApplicationsChart({
  series,
  granularity,
  periods,
  onGranularityChange,
  onPeriodsChange,
}: ApplicationsChartProps) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Default selection to the latest bucket if none active
  const activeIndex = selectedIndex !== null ? selectedIndex : (series.length > 0 ? series.length - 1 : null);
  const activeBucket = activeIndex !== null ? series[activeIndex] : null;

  // Arrow key navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (series.length === 0) return;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const cur = prev !== null ? prev : series.length - 1;
        return Math.max(0, cur - 1);
      });
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const cur = prev !== null ? prev : series.length - 1;
        return Math.min(series.length - 1, cur + 1);
      });
    }
  };

  useEffect(() => {
    // Reset selection if series length changes
    if (selectedIndex !== null && selectedIndex >= series.length) {
      setSelectedIndex(null);
    }
  }, [series.length, selectedIndex]);

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      onKeyDown={handleKeyDown}
      className="border border-[var(--color-ink)] bg-[var(--color-paper)] p-4 flex flex-col justify-between focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] select-none h-full"
    >
      {/* Top Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-ink-mute)]/30 pb-3">
        <div className="flex items-center gap-3">
          <span className="font-mono text-xs font-bold uppercase tracking-wider text-[var(--color-ink)]">
            APPLICATIONS OVER TIME
          </span>
          <span className="font-mono text-xs text-[var(--color-ink-mute)]">
            [←/→ NAVIGATE]
          </span>
        </div>

        {/* Controls: Granularity & Period Range */}
        <div className="flex items-center gap-3">
          {/* Segmented Control */}
          <div className="inline-flex border border-[var(--color-ink)]">
            <button
              type="button"
              onClick={() => onGranularityChange("week")}
              className={`px-2 py-0.5 font-mono text-xs font-bold uppercase transition-none cursor-pointer ${
                granularity === "week"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              WEEK
            </button>
            <button
              type="button"
              onClick={() => onGranularityChange("month")}
              className={`px-2 py-0.5 font-mono text-xs font-bold uppercase transition-none border-l border-[var(--color-ink)] cursor-pointer ${
                granularity === "month"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              MONTH
            </button>
          </div>

          {/* Periods selector */}
          <div className="inline-flex border border-[var(--color-ink)]">
            <button
              type="button"
              onClick={() => onPeriodsChange(12)}
              className={`px-2 py-0.5 font-mono text-xs font-bold uppercase transition-none cursor-pointer ${
                periods === 12
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              12
            </button>
            <button
              type="button"
              onClick={() => onPeriodsChange(24)}
              className={`px-2 py-0.5 font-mono text-xs font-bold uppercase transition-none border-l border-[var(--color-ink)] cursor-pointer ${
                periods === 24
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              24
            </button>
          </div>
        </div>
      </div>

      {/* Visx Chart Canvas */}
      <div className="flex-1 w-full min-h-[220px] py-2">
        <ParentSize>
          {({ width, height }) => (
            <InnerChart
              width={width}
              height={height}
              series={series}
              granularity={granularity}
              selectedIndex={activeIndex}
              onSelectIndex={setSelectedIndex}
            />
          )}
        </ParentSize>
      </div>

      {/* NieR Fixed Readout Bar */}
      <div className="border-t border-[var(--color-ink)] pt-2 mt-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs font-black uppercase text-[var(--color-ink)]">
            READOUT:
          </span>
          <span className="font-mono text-xs font-bold uppercase text-[var(--color-ink)] tracking-wider">
            {activeBucket ? formatReadout(activeBucket, granularity) : "NO DATA"}
          </span>
        </div>
        {activeBucket && (
          <span className="font-mono text-xs text-[var(--color-ink-mute)]">
            {activeBucket.count === 0 ? "ZERO APPLICATIONS" : `${activeBucket.count} FILED`}
          </span>
        )}
      </div>
    </div>
  );
}
