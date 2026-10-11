import { useState, useEffect } from "react";
import { RangeSlider } from "../ui/RangeSlider";
import type { SalaryInterval } from "../../bindings/SalaryInterval";
import {
  MAX_ANNUAL_SALARY,
  STEP_ANNUAL_SALARY,
  MAX_HOURLY_RATE,
  STEP_HOURLY_RATE,
  toHourlyRate,
  toAnnualSalary,
  parseSalaryInput,
  parseHourlyInput,
  formatSalaryNumber,
} from "../../lib/salary";

// Re-export for backward compatibility with existing component imports & tests
export { parseSalaryInput, parseHourlyInput };

export interface SalaryRangeFilterProps {
  salaryMin: number | null;
  salaryMax: number | null;
  currency: string | null;
  interval?: SalaryInterval | null;
  includeUnknown: boolean;
  onChange: (vals: {
    salaryMin: number | null;
    salaryMax: number | null;
    currency: string | null;
    interval: SalaryInterval | null;
    includeUnknown: boolean;
  }) => void;
}

export function SalaryRangeFilter({
  salaryMin,
  salaryMax,
  currency,
  interval = null,
  includeUnknown,
  onChange,
}: SalaryRangeFilterProps) {
  // Determine mode: "annual" vs "hourly"
  const [mode, setMode] = useState<"annual" | "hourly">(() => {
    if (interval === "hour") return "hourly";
    return "annual";
  });

  const isHourly = mode === "hourly";

  // Normalized values for current mode
  const currentHourlyMin = toHourlyRate(salaryMin);
  const currentHourlyMax = toHourlyRate(salaryMax);

  const currentAnnualMin = toAnnualSalary(salaryMin);
  const currentAnnualMax = toAnnualSalary(salaryMax);

  const sliderMin = isHourly ? (currentHourlyMin ?? 0) : (currentAnnualMin ?? 0);
  const sliderMax = isHourly
    ? (currentHourlyMax ?? MAX_HOURLY_RATE)
    : (currentAnnualMax ?? MAX_ANNUAL_SALARY);

  const [minText, setMinText] = useState(() => {
    if (isHourly) return currentHourlyMin != null ? String(currentHourlyMin) : "";
    return currentAnnualMin != null ? formatSalaryNumber(currentAnnualMin) : "";
  });

  const [maxText, setMaxText] = useState(() => {
    if (isHourly) return currentHourlyMax != null ? String(currentHourlyMax) : "";
    return currentAnnualMax != null ? formatSalaryNumber(currentAnnualMax) : "";
  });

  // Keep text inputs in sync with external prop changes
  useEffect(() => {
    if (isHourly) {
      setMinText(currentHourlyMin != null ? String(currentHourlyMin) : "");
      setMaxText(currentHourlyMax != null ? String(currentHourlyMax) : "");
    } else {
      setMinText(currentAnnualMin != null ? formatSalaryNumber(currentAnnualMin) : "");
      setMaxText(currentAnnualMax != null ? formatSalaryNumber(currentAnnualMax) : "");
    }
  }, [salaryMin, salaryMax, isHourly, currentHourlyMin, currentHourlyMax, currentAnnualMin, currentAnnualMax]);

  const handleModeSwitch = (newMode: "annual" | "hourly") => {
    if (newMode === mode) return;
    setMode(newMode);

    if (newMode === "hourly") {
      const hMin = toHourlyRate(currentAnnualMin);
      const hMax = toHourlyRate(currentAnnualMax);
      setMinText(hMin != null ? String(hMin) : "");
      setMaxText(hMax != null ? String(hMax) : "");
      onChange({
        salaryMin: toAnnualSalary(hMin),
        salaryMax: toAnnualSalary(hMax),
        currency,
        interval: "hour",
        includeUnknown,
      });
    } else {
      const aMin = toAnnualSalary(currentHourlyMin);
      const aMax = toAnnualSalary(currentHourlyMax);
      setMinText(aMin != null ? formatSalaryNumber(aMin) : "");
      setMaxText(aMax != null ? formatSalaryNumber(aMax) : "");
      onChange({
        salaryMin: aMin,
        salaryMax: aMax,
        currency,
        interval: null,
        includeUnknown,
      });
    }
  };

  const handleSliderChange = ([newMin, newMax]: [number, number]) => {
    if (isHourly) {
      const finalMin = newMin === 0 ? null : toAnnualSalary(newMin);
      const finalMax = newMax === MAX_HOURLY_RATE ? null : toAnnualSalary(newMax);
      onChange({
        salaryMin: finalMin,
        salaryMax: finalMax,
        currency,
        interval,
        includeUnknown,
      });
    } else {
      const finalMin = newMin === 0 ? null : newMin;
      const finalMax = newMax === MAX_ANNUAL_SALARY ? null : newMax;
      onChange({
        salaryMin: finalMin,
        salaryMax: finalMax,
        currency,
        interval,
        includeUnknown,
      });
    }
  };

  const handleMinBlur = () => {
    if (isHourly) {
      const parsed = parseHourlyInput(minText);
      const validMin = parsed != null ? Math.max(0, Math.min(parsed, currentHourlyMax ?? MAX_HOURLY_RATE)) : null;
      onChange({
        salaryMin: validMin != null ? toAnnualSalary(validMin) : null,
        salaryMax,
        currency,
        interval,
        includeUnknown,
      });
      setMinText(validMin != null ? String(validMin) : "");
    } else {
      const parsed = parseSalaryInput(minText);
      const validMin = parsed != null ? Math.max(0, Math.min(parsed, currentAnnualMax ?? MAX_ANNUAL_SALARY)) : null;
      onChange({
        salaryMin: validMin,
        salaryMax,
        currency,
        interval,
        includeUnknown,
      });
      setMinText(validMin != null ? formatSalaryNumber(validMin) : "");
    }
  };

  const handleMaxBlur = () => {
    if (isHourly) {
      const parsed = parseHourlyInput(maxText);
      const validMax = parsed != null ? Math.max(currentHourlyMin ?? 0, parsed) : null;
      onChange({
        salaryMin,
        salaryMax: validMax != null ? toAnnualSalary(validMax) : null,
        currency,
        interval,
        includeUnknown,
      });
      setMaxText(validMax != null ? String(validMax) : "");
    } else {
      const parsed = parseSalaryInput(maxText);
      const validMax = parsed != null ? Math.max(currentAnnualMin ?? 0, parsed) : null;
      onChange({
        salaryMin,
        salaryMax: validMax,
        currency,
        interval,
        includeUnknown,
      });
      setMaxText(validMax != null ? formatSalaryNumber(validMax) : "");
    }
  };

  const hasFilter = salaryMin != null || salaryMax != null || interval != null;

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      {/* Header and Reset */}
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          {isHourly ? "COMPENSATION (HOURLY RATE)" : "SALARY RANGE (ANNUAL)"}
        </label>
        {hasFilter && (
          <button
            onClick={() =>
              onChange({
                salaryMin: null,
                salaryMax: null,
                currency,
                interval: null,
                includeUnknown,
              })
            }
            className="text-[10px] text-[var(--color-ink-mute)] hover:text-[var(--color-cherry)] uppercase cursor-pointer"
          >
            [RESET]
          </button>
        )}
      </div>

      {/* Mode Switcher: Annual vs Hourly */}
      <div className="flex items-center border border-[var(--color-ink)] p-0.5 bg-[var(--color-paper-2)]/30">
        <button
          type="button"
          onClick={() => handleModeSwitch("annual")}
          className={`flex-1 py-1 text-center font-bold tracking-wider text-[11px] uppercase cursor-pointer border ${
            !isHourly
              ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
              : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
          }`}
        >
          ANNUAL ($/YR)
        </button>
        <button
          type="button"
          onClick={() => handleModeSwitch("hourly")}
          className={`flex-1 py-1 text-center font-bold tracking-wider text-[11px] uppercase cursor-pointer border ${
            isHourly
              ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
              : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
          }`}
        >
          HOURLY ($/HR)
        </button>
      </div>

      {/* Numeric Inputs + Currency */}
      <div className="grid grid-cols-12 gap-2 items-center">
        <div className="col-span-4 flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)]">
          <span className="px-2 text-[var(--color-ink-mute)] text-[10px] font-bold">
            {isHourly ? "MIN $/HR" : "MIN $"}
          </span>
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
          <span className="px-2 text-[var(--color-ink-mute)] text-[10px] font-bold">
            {isHourly ? "MAX $/HR" : "MAX $"}
          </span>
          <input
            type="text"
            value={maxText}
            placeholder={isHourly ? "250+" : "500K+"}
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
                interval,
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
          max={isHourly ? MAX_HOURLY_RATE : MAX_ANNUAL_SALARY}
          step={isHourly ? STEP_HOURLY_RATE : STEP_ANNUAL_SALARY}
          value={[sliderMin, sliderMax]}
          onChange={handleSliderChange}
        />
        <div className="flex justify-between text-[10px] text-[var(--color-ink-mute)] mt-1 font-mono">
          {isHourly ? (
            <>
              <span>$0</span>
              <span>$50</span>
              <span>$100</span>
              <span>$150</span>
              <span>$200</span>
              <span>$250+/HR</span>
            </>
          ) : (
            <>
              <span>$0</span>
              <span>$125K</span>
              <span>$250K</span>
              <span>$375K</span>
              <span>$500K+</span>
            </>
          )}
        </div>
      </div>

      {/* Options: Hourly Restriction & Include Unknown */}
      <div className="flex flex-col gap-1.5 pt-1">
        {isHourly && (
          <label className="flex items-center gap-2 cursor-pointer text-[11px] text-[var(--color-ink)]">
            <input
              type="checkbox"
              checked={interval === "hour"}
              onChange={(e) =>
                onChange({
                  salaryMin,
                  salaryMax,
                  currency,
                  interval: e.target.checked ? "hour" : null,
                  includeUnknown,
                })
              }
              className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
            />
            <span className="uppercase font-bold tracking-wider">
              ONLY POSTINGS SPECIFYING HOURLY RATE
            </span>
          </label>
        )}

        <label className="flex items-center gap-2 cursor-pointer text-[11px] text-[var(--color-ink)]">
          <input
            type="checkbox"
            checked={includeUnknown}
            onChange={(e) =>
              onChange({
                salaryMin,
                salaryMax,
                currency,
                interval,
                includeUnknown: e.target.checked,
              })
            }
            className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
          />
          <span className="uppercase font-bold tracking-wider">
            INCLUDE JOBS WITHOUT STATED {isHourly ? "COMPENSATION" : "SALARY"}
          </span>
        </label>
      </div>
    </div>
  );
}
