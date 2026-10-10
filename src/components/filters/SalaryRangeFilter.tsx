import { useState, useEffect } from "react";
import { RangeSlider } from "../ui/RangeSlider";

export interface SalaryRangeFilterProps {
  salaryMin: number | null;
  salaryMax: number | null;
  currency: string | null;
  includeUnknown: boolean;
  onChange: (vals: {
    salaryMin: number | null;
    salaryMax: number | null;
    currency: string | null;
    includeUnknown: boolean;
  }) => void;
}

const MAX_SLIDER_VAL = 500000;
const SLIDER_STEP = 5000;

function parseSalaryInput(raw: string): number | null {
  const trimmed = raw.trim().toLowerCase().replace(/[$,]/g, "");
  if (!trimmed) return null;
  if (trimmed.endsWith("k")) {
    const num = parseFloat(trimmed.slice(0, -1));
    return isNaN(num) ? null : Math.round(num * 1000);
  }
  const num = parseFloat(trimmed);
  if (isNaN(num)) return null;
  // If user typed e.g. 150, interpret as 150k if <= 500
  if (num > 0 && num <= 500) {
    return Math.round(num * 1000);
  }
  return Math.round(num);
}

function formatDisplay(num: number | null): string {
  if (num == null) return "";
  return num.toLocaleString();
}

export function SalaryRangeFilter({
  salaryMin,
  salaryMax,
  currency,
  includeUnknown,
  onChange,
}: SalaryRangeFilterProps) {
  const currentMin = salaryMin ?? 0;
  const currentMax = salaryMax ?? MAX_SLIDER_VAL;

  const [minText, setMinText] = useState(salaryMin != null ? formatDisplay(salaryMin) : "");
  const [maxText, setMaxText] = useState(salaryMax != null ? formatDisplay(salaryMax) : "");

  useEffect(() => {
    setMinText(salaryMin != null ? formatDisplay(salaryMin) : "");
  }, [salaryMin]);

  useEffect(() => {
    setMaxText(salaryMax != null ? formatDisplay(salaryMax) : "");
  }, [salaryMax]);

  const handleSliderChange = ([newMin, newMax]: [number, number]) => {
    const finalMin = newMin === 0 ? null : newMin;
    const finalMax = newMax === MAX_SLIDER_VAL ? null : newMax;
    onChange({
      salaryMin: finalMin,
      salaryMax: finalMax,
      currency,
      includeUnknown,
    });
  };

  const handleMinBlur = () => {
    const parsed = parseSalaryInput(minText);
    const validMin = parsed != null ? Math.max(0, Math.min(parsed, currentMax)) : null;
    onChange({
      salaryMin: validMin,
      salaryMax,
      currency,
      includeUnknown,
    });
    setMinText(validMin != null ? formatDisplay(validMin) : "");
  };

  const handleMaxBlur = () => {
    const parsed = parseSalaryInput(maxText);
    const validMax = parsed != null ? Math.max(currentMin, parsed) : null;
    onChange({
      salaryMin,
      salaryMax: validMax,
      currency,
      includeUnknown,
    });
    setMaxText(validMax != null ? formatDisplay(validMax) : "");
  };

  const hasFilter = salaryMin != null || salaryMax != null;

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          SALARY RANGE (ANNUAL)
        </label>
        {hasFilter && (
          <button
            onClick={() =>
              onChange({
                salaryMin: null,
                salaryMax: null,
                currency,
                includeUnknown,
              })
            }
            className="text-[10px] text-[var(--color-ink-mute)] hover:text-[var(--color-cherry)] uppercase cursor-pointer"
          >
            [RESET]
          </button>
        )}
      </div>

      {/* Numeric Inputs + Currency */}
      <div className="grid grid-cols-12 gap-2 items-center">
        <div className="col-span-4 flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)]">
          <span className="px-2 text-[var(--color-ink-mute)] text-[10px] font-bold">MIN $</span>
          <input
            type="text"
            value={minText}
            placeholder="0"
            onChange={(e) => setMinText(e.target.value)}
            onBlur={handleMinBlur}
            onKeyDown={(e) => e.key === "Enter" && handleMinBlur()}
            className="w-full py-1 pr-2 bg-transparent text-[var(--color-ink)] text-xs font-mono focus:outline-none"
          />
        </div>

        <div className="col-span-1 text-center text-[var(--color-ink-mute)]">—</div>

        <div className="col-span-4 flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)]">
          <span className="px-2 text-[var(--color-ink-mute)] text-[10px] font-bold">MAX $</span>
          <input
            type="text"
            value={maxText}
            placeholder="500K+"
            onChange={(e) => setMaxText(e.target.value)}
            onBlur={handleMaxBlur}
            onKeyDown={(e) => e.key === "Enter" && handleMaxBlur()}
            className="w-full py-1 pr-2 bg-transparent text-[var(--color-ink)] text-xs font-mono focus:outline-none"
          />
        </div>

        <div className="col-span-3">
          <select
            value={currency || "USD"}
            onChange={(e) =>
              onChange({
                salaryMin,
                salaryMax,
                currency: e.target.value === "ALL" ? null : e.target.value,
                includeUnknown,
              })
            }
            className="w-full py-1 px-1.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] text-xs font-mono uppercase focus:outline-none cursor-pointer"
          >
            <option value="USD">USD ($)</option>
            <option value="CAD">CAD ($)</option>
            <option value="EUR">EUR (€)</option>
            <option value="GBP">GBP (£)</option>
            <option value="ALL">ALL CURR</option>
          </select>
        </div>
      </div>

      {/* Dual Slider */}
      <div className="pt-1 px-1">
        <RangeSlider
          min={0}
          max={MAX_SLIDER_VAL}
          step={SLIDER_STEP}
          value={[currentMin, currentMax]}
          onChange={handleSliderChange}
        />
        <div className="flex justify-between text-[10px] text-[var(--color-ink-mute)] mt-1 font-mono">
          <span>$0</span>
          <span>$125K</span>
          <span>$250K</span>
          <span>$375K</span>
          <span>$500K+</span>
        </div>
      </div>

      {/* Include Unknown Toggle */}
      <div className="flex items-center gap-2 pt-1">
        <label className="flex items-center gap-2 cursor-pointer text-[11px] text-[var(--color-ink)]">
          <input
            type="checkbox"
            checked={includeUnknown}
            onChange={(e) =>
              onChange({
                salaryMin,
                salaryMax,
                currency,
                includeUnknown: e.target.checked,
              })
            }
            className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
          />
          <span className="uppercase font-bold tracking-wider">
            INCLUDE JOBS WITHOUT STATED SALARY
          </span>
        </label>
      </div>
    </div>
  );
}
