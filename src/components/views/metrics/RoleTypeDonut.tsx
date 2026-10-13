import { useState, useMemo } from "react";
import { Pie } from "@visx/shape";
import { Group } from "@visx/group";
import { ParentSize } from "@visx/responsive";
import type { RoleTypeMetric } from "../../../bindings/RoleTypeMetric";
import type { RoleType } from "../../../bindings/RoleType";

interface RoleTypeDonutProps {
  data: RoleTypeMetric[];
  className?: string;
}

export function formatRoleType(rt: RoleType): string {
  switch (rt) {
    case "full_time":
      return "FULL-TIME";
    case "contract":
      return "CONTRACT";
    case "internship":
      return "INTERNSHIP";
    case "part_time":
      return "PART-TIME";
    case "seasonal":
      return "SEASONAL";
    case "unknown":
      return "UNKNOWN";
  }
}

function getPatternId(rt: RoleType): string {
  switch (rt) {
    case "full_time":
      return "role-solid";
    case "contract":
      return "role-hatch";
    case "internship":
      return "role-dots";
    case "part_time":
      return "role-checker";
    case "seasonal":
      return "role-grid";
    case "unknown":
      return "role-paper";
  }
}

interface InnerDonutProps {
  width: number;
  height: number;
  data: RoleTypeMetric[];
  selectedRole: RoleType | null;
  onSelectRole: (role: RoleType | null) => void;
}

function InnerDonut({
  width,
  height,
  data,
  selectedRole,
  onSelectRole,
}: InnerDonutProps) {
  const size = Math.min(width, height);
  const radius = Math.floor(size / 2);
  const innerRadius = Math.floor(radius * 0.55);
  const centerX = Math.floor(width / 2);
  const centerY = Math.floor(height / 2);

  const activeData = useMemo(() => {
    return data.filter((d) => d.count > 0);
  }, [data]);

  return (
    <svg
      width={width}
      height={height}
      style={{ overflow: "visible" }}
      shapeRendering="crispEdges"
    >
      <defs>
        {/* Hatch Pattern */}
        <pattern
          id="role-hatch"
          width="6"
          height="6"
          patternTransform="rotate(45)"
          patternUnits="userSpaceOnUse"
        >
          <rect width="6" height="6" fill="var(--color-paper)" />
          <line
            x1="0"
            y1="0"
            x2="0"
            y2="6"
            stroke="var(--color-ink)"
            strokeWidth="2"
          />
        </pattern>

        {/* Dots Pattern */}
        <pattern
          id="role-dots"
          width="6"
          height="6"
          patternUnits="userSpaceOnUse"
        >
          <rect width="6" height="6" fill="var(--color-paper)" />
          <circle cx="3" cy="3" r="1.5" fill="var(--color-ink)" />
        </pattern>

        {/* Checker Pattern */}
        <pattern
          id="role-checker"
          width="6"
          height="6"
          patternUnits="userSpaceOnUse"
        >
          <rect width="6" height="6" fill="var(--color-paper)" />
          <rect width="3" height="3" fill="var(--color-ink)" />
          <rect x="3" y="3" width="3" height="3" fill="var(--color-ink)" />
        </pattern>

        {/* Grid Pattern */}
        <pattern
          id="role-grid"
          width="6"
          height="6"
          patternUnits="userSpaceOnUse"
        >
          <rect width="6" height="6" fill="var(--color-paper)" />
          <path
            d="M 6 0 L 0 0 0 6"
            fill="none"
            stroke="var(--color-ink)"
            strokeWidth="1"
          />
        </pattern>
      </defs>

      <Group top={centerY} left={centerX}>
        {activeData.length > 0 ? (
          <Pie<RoleTypeMetric>
            data={activeData}
            pieValue={(d) => d.count}
            outerRadius={radius}
            innerRadius={innerRadius}
            padAngle={0}
            pieSort={null}
          >
            {(pie) =>
              pie.arcs.map((arc) => {
                const rt = arc.data.role_type;
                const isSelected = selectedRole === rt;
                const patternId = getPatternId(rt);
                const fillValue =
                  patternId === "role-solid"
                    ? "var(--color-ink)"
                    : patternId === "role-paper"
                    ? "var(--color-paper)"
                    : `url(#${patternId})`;

                return (
                  <path
                    key={`slice-${rt}`}
                    d={pie.path(arc) ?? undefined}
                    fill={fillValue}
                    stroke="var(--color-ink)"
                    strokeWidth={isSelected ? 2.5 : 1}
                    onMouseEnter={() => onSelectRole(rt)}
                    className="cursor-pointer"
                  />
                );
              })
            }
          </Pie>
        ) : (
          /* Empty donut placeholder */
          <g>
            <circle
              r={radius}
              fill="none"
              stroke="var(--color-ink-mute)"
              strokeWidth={1}
              strokeDasharray="3 3"
            />
            <circle
              r={innerRadius}
              fill="none"
              stroke="var(--color-ink-mute)"
              strokeWidth={1}
            />
          </g>
        )}
      </Group>
    </svg>
  );
}

export function RoleTypeDonut({ data, className = "" }: RoleTypeDonutProps) {
  const [selectedRole, setSelectedRole] = useState<RoleType | null>(null);

  const totalApplied = useMemo(
    () => data.reduce((sum, d) => sum + d.count, 0),
    [data]
  );

  const activeRole =
    selectedRole !== null
      ? selectedRole
      : data.find((d) => d.count > 0)?.role_type ?? null;

  const activeItem = activeRole
    ? data.find((d) => d.role_type === activeRole)
    : null;

  return (
    <div
      className={`border border-[var(--color-ink)] bg-[var(--color-paper)] p-4 flex flex-col justify-between select-none h-full ${className}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between border-b border-[var(--color-ink-mute)]/30 pb-3">
        <span className="font-mono text-xs font-bold uppercase tracking-wider text-[var(--color-ink)]">
          JOB TYPES APPLIED
        </span>
        <span className="font-mono text-xs text-[var(--color-ink-mute)]">
          {totalApplied} TOTAL
        </span>
      </div>

      {/* Donut Chart Visual */}
      <div className="w-full h-[160px] py-2 flex items-center justify-center">
        <ParentSize>
          {({ width, height }) => (
            <InnerDonut
              width={width}
              height={height}
              data={data}
              selectedRole={activeRole}
              onSelectRole={setSelectedRole}
            />
          )}
        </ParentSize>
      </div>

      {/* Legend Table */}
      <div className="border-t border-[var(--color-ink-mute)]/30 pt-2 flex flex-col gap-1">
        {data.map((item) => {
          const isSelected = activeRole === item.role_type;
          const pct =
            totalApplied > 0
              ? Math.round((item.count / totalApplied) * 100)
              : 0;
          const patternId = getPatternId(item.role_type);

          return (
            <div
              key={`legend-${item.role_type}`}
              onClick={() => setSelectedRole(item.role_type)}
              onMouseEnter={() => setSelectedRole(item.role_type)}
              className={`flex items-center justify-between px-1.5 py-0.5 cursor-pointer font-mono text-xs transition-none ${
                isSelected
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                  : "hover:bg-[var(--color-paper-2)] text-[var(--color-ink)]"
              }`}
            >
              <div className="flex items-center gap-2">
                {/* Visual Swatch */}
                <div
                  className={`w-3 h-3 border border-[var(--color-ink)] ${
                    patternId === "role-solid"
                      ? "bg-[var(--color-ink)]"
                      : patternId === "role-hatch"
                      ? "dither-lines"
                      : patternId === "role-dots"
                      ? "dither-dots"
                      : patternId === "role-checker"
                      ? "dither-checker"
                      : patternId === "role-grid"
                      ? "bg-[var(--color-ink)] opacity-50"
                      : "bg-[var(--color-paper)]"
                  }`}
                />
                <span className="uppercase">{formatRoleType(item.role_type)}</span>
              </div>
              <div className="flex items-center gap-3 tabular-nums">
                <span>{item.count}</span>
                <span className="w-8 text-right opacity-70">{pct}%</span>
              </div>
            </div>
          );
        })}
      </div>

      {/* NieR Readout */}
      <div className="border-t border-[var(--color-ink)] pt-2 mt-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs font-black uppercase text-[var(--color-ink)]">
            READOUT:
          </span>
          <span className="font-mono text-xs font-bold uppercase text-[var(--color-ink)]">
            {activeItem
              ? `${formatRoleType(activeItem.role_type)} · ${activeItem.count} APPLIED (${
                  totalApplied > 0
                    ? Math.round((activeItem.count / totalApplied) * 100)
                    : 0
                }%)`
              : "NO APPLIED JOBS"}
          </span>
        </div>
      </div>
    </div>
  );
}
