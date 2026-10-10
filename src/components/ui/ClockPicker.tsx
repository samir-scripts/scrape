import React, { useState, useEffect, useRef, useCallback, useMemo } from "react";
import {
  type ClockCadence,
  type AmPm,
  type ClockScheduleState,
  CLOCK_CADENCE_OPTIONS,
  pad2,
  clockToCron,
} from "../../lib/cronClock";

export type ClockMode = "hours" | "minutes";

export interface ClockPickerProps {
  value?: ClockScheduleState;
  onChange?: (state: ClockScheduleState, cronExpr: string) => void;
  className?: string;
}

const VIEWBOX_SIZE = 260;
const CENTER = VIEWBOX_SIZE / 2; // 130
const DIAL_RADIUS = 88;
const MINOR_TICK_R1 = 114;
const MINOR_TICK_R2 = 119;
const MAJOR_TICK_R1 = 110;
const MAJOR_TICK_R2 = 120;

export function ClockPicker({ value, onChange, className = "" }: ClockPickerProps) {
  const [hour12, setHour12] = useState<number>(value?.hour12 ?? 9);
  const [minute, setMinute] = useState<number>(value?.minute ?? 0);
  const [ampm, setAmPm] = useState<AmPm>(value?.ampm ?? "AM");
  const [cadence, setCadence] = useState<ClockCadence>(value?.cadence ?? "daily");
  const [mode, setMode] = useState<ClockMode>("hours");
  const [isDragging, setIsDragging] = useState<boolean>(false);

  const svgRef = useRef<SVGSVGElement | null>(null);

  // Synchronize when value changes externally
  useEffect(() => {
    if (value) {
      if (value.hour12 !== undefined) setHour12(value.hour12);
      if (value.minute !== undefined) setMinute(value.minute);
      if (value.ampm !== undefined) setAmPm(value.ampm);
      if (value.cadence !== undefined) setCadence(value.cadence);
    }
  }, [value?.hour12, value?.minute, value?.ampm, value?.cadence]);

  // Compute live cron expression
  const cronExpr = useMemo(() => {
    return clockToCron(hour12, minute, ampm, cadence);
  }, [hour12, minute, ampm, cadence]);

  const notifyChange = useCallback(
    (h: number, m: number, ap: AmPm, cad: ClockCadence) => {
      if (!onChange) return;
      const state: ClockScheduleState = {
        hour12: h,
        minute: m,
        ampm: ap,
        cadence: cad,
      };
      onChange(state, clockToCron(h, m, ap, cad));
    },
    [onChange]
  );

  // Convert pointer event to degrees from 12 o'clock [0, 360)
  const getAngleFromEvent = useCallback((e: React.PointerEvent<SVGSVGElement>): number => {
    if (!svgRef.current) return 0;
    const rect = svgRef.current.getBoundingClientRect();
    const scaleX = rect.width > 0 ? VIEWBOX_SIZE / rect.width : 1;
    const scaleY = rect.height > 0 ? VIEWBOX_SIZE / rect.height : 1;
    const dx = (e.clientX - rect.left) * scaleX - CENTER;
    const dy = (e.clientY - rect.top) * scaleY - CENTER;

    let rad = Math.atan2(dx, -dy);
    if (rad < 0) rad += 2 * Math.PI;
    return (rad * 180) / Math.PI;
  }, []);

  const updateFromPointer = useCallback(
    (deg: number) => {
      if (mode === "hours") {
        const step = 360 / 12;
        let h = Math.round(deg / step) % 12;
        if (h === 0) h = 12;
        setHour12(h);
        notifyChange(h, minute, ampm, cadence);
      } else {
        const step = 360 / 60;
        const m = Math.round(deg / step) % 60;
        setMinute(m);
        notifyChange(hour12, m, ampm, cadence);
      }
    },
    [mode, minute, hour12, ampm, cadence, notifyChange]
  );

  const handlePointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
    e.preventDefault();
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Ignore if not supported in test environment
    }
    setIsDragging(true);
    const deg = getAngleFromEvent(e);
    updateFromPointer(deg);
  };

  const handlePointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!isDragging) return;
    const deg = getAngleFromEvent(e);
    updateFromPointer(deg);
  };

  const handlePointerUp = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!isDragging) return;
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // Ignored
    }
    setIsDragging(false);

    // Auto-advance to minutes mode when selecting an hour
    if (mode === "hours") {
      setMode("minutes");
    }
  };

  // Keyboard navigation for time adjustment
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowUp" || e.key === "ArrowRight") {
      e.preventDefault();
      if (mode === "hours") {
        const next = (hour12 % 12) + 1;
        setHour12(next);
        notifyChange(next, minute, ampm, cadence);
      } else {
        const next = (minute + 1) % 60;
        setMinute(next);
        notifyChange(hour12, next, ampm, cadence);
      }
    } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
      e.preventDefault();
      if (mode === "hours") {
        const prev = hour12 === 1 ? 12 : hour12 - 1;
        setHour12(prev);
        notifyChange(prev, minute, ampm, cadence);
      } else {
        const prev = minute === 0 ? 59 : minute - 1;
        setMinute(prev);
        notifyChange(hour12, prev, ampm, cadence);
      }
    }
  };

  // Coordinates for the active hand needle and selector reticle
  const activeAngleDeg = mode === "hours" ? (hour12 % 12) * 30 : minute * 6;
  const activeAngleRad = (activeAngleDeg * Math.PI) / 180;
  const handTipX = CENTER + DIAL_RADIUS * Math.sin(activeAngleRad);
  const handTipY = CENTER - DIAL_RADIUS * Math.cos(activeAngleRad);

  // Generate 60 ticks along perimeter
  const rimTicks = useMemo(() => {
    const ticks = [];
    for (let i = 0; i < 60; i++) {
      const isMajor = i % 5 === 0;
      const angle = (i * 6 * Math.PI) / 180;
      const sin = Math.sin(angle);
      const cos = Math.cos(angle);
      const r1 = isMajor ? MAJOR_TICK_R1 : MINOR_TICK_R1;
      const r2 = isMajor ? MAJOR_TICK_R2 : MINOR_TICK_R2;
      ticks.push({
        key: i,
        x1: CENTER + r1 * sin,
        y1: CENTER - r1 * cos,
        x2: CENTER + r2 * sin,
        y2: CENTER - r2 * cos,
        isMajor,
      });
    }
    return ticks;
  }, []);

  const activeDescription = useMemo(() => {
    const opt = CLOCK_CADENCE_OPTIONS.find((c) => c.id === cadence);
    return opt ? opt.description(hour12, minute, ampm) : "";
  }, [cadence, hour12, minute, ampm]);

  return (
    <div
      className={`flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] font-mono select-none w-full max-w-md ${className}`}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      aria-label="Clock Schedule Picker"
    >
      {/* Telemetry Header */}
      <div className="flex items-center justify-between text-[10px] tracking-widest text-[var(--color-ink-mute)] border-b border-[var(--color-ink)] pb-1.5">
        <span>SYS//TIME_PICKER_CLOCK</span>
        <span className="flex items-center gap-1 font-bold">
          <span className="inline-block w-1.5 h-1.5 bg-[var(--color-ink)]" />
          {mode === "hours" ? "ACTIVE: HOUR" : "ACTIVE: MINUTE"}
        </span>
      </div>

      {/* Digital Readout Bar & AM/PM Toggle */}
      <div className="flex items-center justify-between gap-3 bg-[var(--color-paper-2)]/40 p-2.5 border border-[var(--color-ink)]">
        {/* HH : MM Selectors */}
        <div className="flex items-center gap-1.5 text-3xl font-black tracking-tight">
          <button
            type="button"
            onClick={() => setMode("hours")}
            className={`px-2.5 py-1 border cursor-pointer transition-colors ${
              mode === "hours"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                : "bg-transparent text-[var(--color-ink)] border-transparent hover:border-[var(--color-ink)]"
            }`}
            aria-label="Select hour"
          >
            {pad2(hour12)}
          </button>
          <span className="text-[var(--color-ink)] opacity-70 animate-pulse font-bold">:</span>
          <button
            type="button"
            onClick={() => setMode("minutes")}
            className={`px-2.5 py-1 border cursor-pointer transition-colors ${
              mode === "minutes"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                : "bg-transparent text-[var(--color-ink)] border-transparent hover:border-[var(--color-ink)]"
            }`}
            aria-label="Select minute"
          >
            {pad2(minute)}
          </button>
        </div>

        {/* AM / PM Segmented Switch */}
        <div className="flex flex-col border border-[var(--color-ink)] text-xs font-bold">
          <button
            type="button"
            onClick={() => {
              setAmPm("AM");
              notifyChange(hour12, minute, "AM", cadence);
            }}
            className={`px-3 py-1 cursor-pointer transition-colors ${
              ampm === "AM"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                : "bg-transparent text-[var(--color-ink-mute)] hover:text-[var(--color-ink)]"
            }`}
          >
            AM
          </button>
          <div className="h-[1px] bg-[var(--color-ink)]" />
          <button
            type="button"
            onClick={() => {
              setAmPm("PM");
              notifyChange(hour12, minute, "PM", cadence);
            }}
            className={`px-3 py-1 cursor-pointer transition-colors ${
              ampm === "PM"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                : "bg-transparent text-[var(--color-ink-mute)] hover:text-[var(--color-ink)]"
            }`}
          >
            PM
          </button>
        </div>
      </div>

      {/* SVG Clock Dial with Corner Reticles */}
      <div className="relative flex justify-center items-center py-2 bg-[var(--color-paper)]">
        {/* Tactical Corner Brackets */}
        <div className="absolute top-0 left-0 w-2.5 h-2.5 border-t border-l border-[var(--color-ink)] pointer-events-none" />
        <div className="absolute top-0 right-0 w-2.5 h-2.5 border-t border-r border-[var(--color-ink)] pointer-events-none" />
        <div className="absolute bottom-0 left-0 w-2.5 h-2.5 border-b border-l border-[var(--color-ink)] pointer-events-none" />
        <div className="absolute bottom-0 right-0 w-2.5 h-2.5 border-b border-r border-[var(--color-ink)] pointer-events-none" />

        <svg
          ref={svgRef}
          viewBox={`0 0 ${VIEWBOX_SIZE} ${VIEWBOX_SIZE}`}
          className="w-64 h-64 select-none touch-none cursor-crosshair"
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          shapeRendering="geometricPrecision"
          data-testid="clock-dial-svg"
        >
          {/* Dial Circle Frame */}
          <circle
            cx={CENTER}
            cy={CENTER}
            r={124}
            fill="none"
            stroke="var(--color-ink)"
            strokeWidth="1"
          />
          <circle
            cx={CENTER}
            cy={CENTER}
            r={105}
            fill="none"
            stroke="var(--color-paper-2)"
            strokeWidth="1"
            strokeDasharray="2 4"
          />

          {/* Crosshair Guidelines at Cardinal Points */}
          <line
            x1={CENTER}
            y1={14}
            x2={CENTER}
            y2={VIEWBOX_SIZE - 14}
            stroke="var(--color-ink-mute)"
            strokeWidth="0.5"
            strokeDasharray="1 3"
          />
          <line
            x1={14}
            y1={CENTER}
            x2={VIEWBOX_SIZE - 14}
            y2={CENTER}
            stroke="var(--color-ink-mute)"
            strokeWidth="0.5"
            strokeDasharray="1 3"
          />

          {/* Radial Rim Ticks */}
          {rimTicks.map((tick) => (
            <line
              key={tick.key}
              x1={tick.x1}
              y1={tick.y1}
              x2={tick.x2}
              y2={tick.y2}
              stroke={tick.isMajor ? "var(--color-ink)" : "var(--color-ink-mute)"}
              strokeWidth={tick.isMajor ? "1.5" : "0.75"}
            />
          ))}

          {/* Hand Needle */}
          <line
            x1={CENTER}
            y1={CENTER}
            x2={handTipX}
            y2={handTipY}
            stroke="var(--color-ink)"
            strokeWidth="1.5"
          />

          {/* Center Hub Pip */}
          <rect
            x={CENTER - 3.5}
            y={CENTER - 3.5}
            width={7}
            height={7}
            fill="var(--color-ink)"
          />

          {/* Selector Reticle at Hand Tip */}
          <rect
            x={handTipX - 13}
            y={handTipY - 13}
            width={26}
            height={26}
            fill="var(--color-ink)"
            stroke="var(--color-paper)"
            strokeWidth="1"
          />

          {/* Dial Numbers */}
          {mode === "hours"
            ? [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].map((h) => {
                const angle = (h * 30 * Math.PI) / 180;
                const nx = CENTER + DIAL_RADIUS * Math.sin(angle);
                const ny = CENTER - DIAL_RADIUS * Math.cos(angle);
                const isSelected = h === hour12;
                return (
                  <text
                    key={h}
                    x={nx}
                    y={ny}
                    textAnchor="middle"
                    dominantBaseline="central"
                    className="font-mono text-xs font-bold pointer-events-none select-none"
                    fill={isSelected ? "var(--color-paper)" : "var(--color-ink)"}
                  >
                    {pad2(h)}
                  </text>
                );
              })
            : [0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55].map((m) => {
                const angle = (m * 6 * Math.PI) / 180;
                const nx = CENTER + DIAL_RADIUS * Math.sin(angle);
                const ny = CENTER - DIAL_RADIUS * Math.cos(angle);
                const isSelected = m === minute;
                return (
                  <text
                    key={m}
                    x={nx}
                    y={ny}
                    textAnchor="middle"
                    dominantBaseline="central"
                    className="font-mono text-[11px] font-bold pointer-events-none select-none"
                    fill={isSelected ? "var(--color-paper)" : "var(--color-ink)"}
                  >
                    {pad2(m)}
                  </text>
                );
              })}

          {/* Display exact minute value inside pip if not a 5-minute step */}
          {mode === "minutes" && minute % 5 !== 0 && (
            <text
              x={handTipX}
              y={handTipY}
              textAnchor="middle"
              dominantBaseline="central"
              className="font-mono text-[10px] font-bold pointer-events-none select-none"
              fill="var(--color-paper)"
            >
              {pad2(minute)}
            </text>
          )}
        </svg>
      </div>

      {/* Cadence Selection */}
      <div className="flex flex-col gap-2 pt-2 border-t border-[var(--color-ink)]">
        <label className="text-[10px] font-bold uppercase tracking-wider text-[var(--color-ink-mute)]">
          EXECUTION CADENCE
        </label>
        <div className="grid grid-cols-3 gap-1.5">
          {CLOCK_CADENCE_OPTIONS.map((opt) => {
            const isSel = opt.id === cadence;
            return (
              <button
                key={opt.id}
                type="button"
                onClick={() => {
                  setCadence(opt.id);
                  notifyChange(hour12, minute, ampm, opt.id);
                }}
                className={`px-2 py-1.5 text-[10px] font-bold uppercase border cursor-pointer text-center truncate transition-colors ${
                  isSel
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-transparent text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>

        {/* Live Target Cron & Description Readout */}
        <div className="mt-1 p-2 bg-[var(--color-paper-2)]/30 border border-[var(--color-ink)] text-[11px]">
          <div className="flex justify-between items-center text-[var(--color-ink)] font-bold">
            <span className="text-[10px] text-[var(--color-ink-mute)]">TARGET CRON:</span>
            <code className="bg-[var(--color-paper)] px-1.5 py-0.5 border border-[var(--color-ink)] text-xs">
              {cronExpr}
            </code>
          </div>
          <div className="text-[10px] text-[var(--color-ink-mute)] mt-1.5">
            SCHEDULE:{" "}
            <span className="text-[var(--color-ink)] font-bold">
              {activeDescription}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
