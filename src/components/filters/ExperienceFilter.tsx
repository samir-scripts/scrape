import React from "react";

export interface ExperienceFilterProps {
  experienceMin: number | null;
  experienceMax: number | null;
  newGrad: string | null; // "any" | "only" | "exclude" | null
  includeUnknown: boolean;
  onChange: (vals: {
    experienceMin: number | null;
    experienceMax: number | null;
    newGrad: string | null;
    includeUnknown: boolean;
  }) => void;
}

export const CAREER_LEVEL_PRESETS: { label: string; min: number | null; max: number | null }[] = [
  { label: "ALL", min: null, max: null },
  { label: "NEW GRAD (0-1)", min: 0, max: 1 },
  { label: "MID (1-3)", min: 1, max: 3 },
  { label: "SENIOR (3-5)", min: 3, max: 5 },
  { label: "STAFF+ (5+)", min: 5, max: null },
];

export const NEW_GRAD_OPTIONS: { id: string; label: string }[] = [
  { id: "any", label: "ANY" },
  { id: "only", label: "NEW GRAD ONLY" },
  { id: "exclude", label: "EXCLUDE NEW GRAD" },
];

export function ExperienceFilter({
  experienceMin,
  experienceMax,
  newGrad = "any",
  includeUnknown,
  onChange,
}: ExperienceFilterProps) {
  const currentNewGrad = newGrad || "any";

  const isPresetActive = (min: number | null, max: number | null) => {
    return experienceMin === min && experienceMax === max;
  };

  const handleMinChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value.trim();
    const num = val === "" ? null : Math.max(0, parseInt(val, 10));
    onChange({
      experienceMin: isNaN(num as number) ? null : num,
      experienceMax,
      newGrad: currentNewGrad,
      includeUnknown,
    });
  };

  const handleMaxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value.trim();
    const num = val === "" ? null : Math.max(0, parseInt(val, 10));
    onChange({
      experienceMin,
      experienceMax: isNaN(num as number) ? null : num,
      newGrad: currentNewGrad,
      includeUnknown,
    });
  };

  const handlePresetClick = (min: number | null, max: number | null) => {
    onChange({
      experienceMin: min,
      experienceMax: max,
      newGrad: currentNewGrad,
      includeUnknown,
    });
  };

  const handleNewGradChange = (val: string) => {
    onChange({
      experienceMin,
      experienceMax,
      newGrad: val,
      includeUnknown,
    });
  };

  const hasFilter =
    experienceMin != null ||
    experienceMax != null ||
    currentNewGrad !== "any";

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          YEARS OF EXPERIENCE & LEVEL
        </label>
        {hasFilter && (
          <button
            onClick={() =>
              onChange({
                experienceMin: null,
                experienceMax: null,
                newGrad: "any",
                includeUnknown,
              })
            }
            className="text-[10px] text-[var(--color-ink-mute)] hover:text-[var(--color-cherry)] uppercase cursor-pointer"
          >
            [RESET]
          </button>
        )}
      </div>

      {/* Preset Career Levels */}
      <div className="flex flex-col gap-1">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          CAREER LEVEL PRESETS
        </span>
        <div className="flex flex-wrap gap-1.5">
          {CAREER_LEVEL_PRESETS.map((p) => {
            const active = isPresetActive(p.min, p.max);
            return (
              <button
                key={p.label}
                type="button"
                onClick={() => handlePresetClick(p.min, p.max)}
                className={`px-2 py-1 text-[11px] font-bold uppercase tracking-wider border cursor-pointer ${
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
      </div>

      {/* Custom Numerical Range */}
      <div className="flex items-center gap-2 pt-0.5">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          CUSTOM:
        </span>
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

      {/* New Grad Qualification Eligibility */}
      <div className="flex flex-col gap-1 pt-0.5">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          NEW GRAD STATUS
        </span>
        <div className="flex gap-1.5">
          {NEW_GRAD_OPTIONS.map((opt) => {
            const active = currentNewGrad === opt.id;
            return (
              <button
                key={opt.id}
                type="button"
                onClick={() => handleNewGradChange(opt.id)}
                className={`flex-1 py-1 px-2 text-[10px] font-bold uppercase tracking-wider border cursor-pointer ${
                  active
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
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
                newGrad: currentNewGrad,
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
