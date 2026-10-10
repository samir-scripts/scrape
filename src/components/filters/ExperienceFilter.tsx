import React from "react";

export interface ExperienceFilterProps {
  experienceMin: number | null;
  experienceMax: number | null;
  includeUnknown: boolean;
  onChange: (vals: {
    experienceMin: number | null;
    experienceMax: number | null;
    includeUnknown: boolean;
  }) => void;
}

const PRESETS: { label: string; min: number | null; max: number | null }[] = [
  { label: "ALL", min: null, max: null },
  { label: "0-1 ENTRY", min: 0, max: 1 },
  { label: "1-3 MID", min: 1, max: 3 },
  { label: "3-5 SENIOR", min: 3, max: 5 },
  { label: "5+ STAFF+", min: 5, max: null },
];

export function ExperienceFilter({
  experienceMin,
  experienceMax,
  includeUnknown,
  onChange,
}: ExperienceFilterProps) {
  const isPresetActive = (min: number | null, max: number | null) => {
    return experienceMin === min && experienceMax === max;
  };

  const handleMinChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value.trim();
    const num = val === "" ? null : Math.max(0, parseInt(val, 10));
    onChange({
      experienceMin: isNaN(num as number) ? null : num,
      experienceMax,
      includeUnknown,
    });
  };

  const handleMaxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value.trim();
    const num = val === "" ? null : Math.max(0, parseInt(val, 10));
    onChange({
      experienceMin,
      experienceMax: isNaN(num as number) ? null : num,
      includeUnknown,
    });
  };

  const hasFilter = experienceMin != null || experienceMax != null;

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          YEARS OF EXPERIENCE (YOE)
        </label>
        {hasFilter && (
          <button
            onClick={() =>
              onChange({
                experienceMin: null,
                experienceMax: null,
                includeUnknown,
              })
            }
            className="text-[10px] text-[var(--color-ink-mute)] hover:text-[var(--color-cherry)] uppercase cursor-pointer"
          >
            [RESET]
          </button>
        )}
      </div>

      {/* Preset Buttons */}
      <div className="flex flex-wrap gap-1.5">
        {PRESETS.map((p) => {
          const active = isPresetActive(p.min, p.max);
          return (
            <button
              key={p.label}
              type="button"
              onClick={() =>
                onChange({
                  experienceMin: p.min,
                  experienceMax: p.max,
                  includeUnknown,
                })
              }
              className={`px-2.5 py-1 text-[11px] font-bold uppercase tracking-wider border cursor-pointer ${
                active
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              {p.label}
            </button>
          );
        })}
      </div>

      {/* Custom Inputs */}
      <div className="flex items-center gap-2 pt-0.5">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">CUSTOM:</span>
        <div className="flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 py-0.5 w-24">
          <span className="text-[10px] text-[var(--color-ink-mute)] mr-1">MIN</span>
          <input
            type="number"
            min={0}
            max={30}
            value={experienceMin ?? ""}
            placeholder="0"
            onChange={handleMinChange}
            className="w-full bg-transparent text-[var(--color-ink)] text-xs font-mono focus:outline-none"
          />
        </div>
        <span className="text-[var(--color-ink-mute)]">—</span>
        <div className="flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 py-0.5 w-24">
          <span className="text-[10px] text-[var(--color-ink-mute)] mr-1">MAX</span>
          <input
            type="number"
            min={0}
            max={30}
            value={experienceMax ?? ""}
            placeholder="ANY"
            onChange={handleMaxChange}
            className="w-full bg-transparent text-[var(--color-ink)] text-xs font-mono focus:outline-none"
          />
        </div>
        <span className="text-[10px] text-[var(--color-ink-mute)] font-bold">YOE</span>
      </div>

      {/* Include Unknown Toggle */}
      <div className="flex items-center gap-2 pt-0.5">
        <label className="flex items-center gap-2 cursor-pointer text-[11px] text-[var(--color-ink)]">
          <input
            type="checkbox"
            checked={includeUnknown}
            onChange={(e) =>
              onChange({
                experienceMin,
                experienceMax,
                includeUnknown: e.target.checked,
              })
            }
            className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
          />
          <span className="uppercase font-bold tracking-wider">
            INCLUDE JOBS WITHOUT STATED YOE
          </span>
        </label>
      </div>
    </div>
  );
}
