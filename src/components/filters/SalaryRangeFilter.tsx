import { useEffect, useState } from "react";
import type { SalaryInterval } from "../../bindings/SalaryInterval";
import { parseSalaryInput, parseHourlyInput } from "../../lib/salary";
export { parseSalaryInput, parseHourlyInput };

export interface SalaryRangeFilterProps {
  salaryMin: number | null; salaryMax: number | null; currency: string | null;
  interval?: SalaryInterval | null; includeUnknown: boolean;
  onDraftChange?: () => void;
  onChange: (values: { salaryMin: number | null; salaryMax: number | null; currency: string | null; interval: SalaryInterval | null; includeUnknown: boolean }) => void;
}
export function SalaryRangeFilter({ salaryMin, salaryMax, currency, interval = null, includeUnknown, onChange, onDraftChange }: SalaryRangeFilterProps) {
  const [minText, setMinText] = useState(salaryMin?.toString() || "");
  const [maxText, setMaxText] = useState(salaryMax?.toString() || "");
  const [error, setError] = useState("");
  const mode = interval === "hour" ? "hour" : "year";
  useEffect(() => { setMinText(salaryMin?.toString() || ""); setMaxText(salaryMax?.toString() || ""); setError(""); }, [salaryMin, salaryMax, interval]);
  const apply = () => {
    const parse = mode === "hour" ? parseHourlyInput : parseSalaryInput;
    const min = parse(minText), max = parse(maxText);
    if ((minText.trim() && min == null) || (maxText.trim() && max == null)) { setError("Enter a nonnegative amount with at most two decimal places."); return; }
    if (min != null && max != null && min > max) { setError("Minimum pay must not exceed maximum pay."); return; }
    setError("");
    onChange({ salaryMin: min, salaryMax: max, currency, interval: min == null && max == null ? interval : mode, includeUnknown });
  };
  const changeMode = (next: "hour" | "year") => {
    setMinText(""); setMaxText(""); setError("");
    onChange({ salaryMin: null, salaryMax: null, currency, interval: next, includeUnknown });
  };
  return <fieldset className="flex flex-col gap-3 text-sm">
    <legend className="font-bold mb-2">Compensation</legend>
    <div className="flex gap-2">
      <button type="button" aria-pressed={mode === "year"} onClick={() => changeMode("year")} className="border px-3 py-2">Annual</button>
      <button type="button" aria-pressed={mode === "hour"} onClick={() => changeMode("hour")} className="border px-3 py-2">Hourly</button>
    </div>
    <p className="text-[var(--color-ink-mute)]">Matches explicitly {mode === "hour" ? "hourly" : "annual"} pay. Switching units clears the amount bounds.</p>
    <div className="flex gap-3">
      <label className="flex flex-col gap-1 min-w-0">Minimum {mode === "hour" ? "per hour" : "per year"}
        <input inputMode="decimal" value={minText} onChange={(event) => { setMinText(event.target.value); onDraftChange?.(); }} onBlur={apply} aria-invalid={!!error} aria-describedby={error ? "salary-filter-error" : undefined} className="w-full border bg-[var(--color-paper)] p-2" />
      </label>
      <label className="flex flex-col gap-1 min-w-0">Maximum {mode === "hour" ? "per hour" : "per year"}
        <input inputMode="decimal" value={maxText} onChange={(event) => { setMaxText(event.target.value); onDraftChange?.(); }} onBlur={apply} aria-invalid={!!error} aria-describedby={error ? "salary-filter-error" : undefined} className="w-full border bg-[var(--color-paper)] p-2" />
      </label>
    </div>
    {error && <p id="salary-filter-error" role="alert" className="text-[var(--color-danger)]">{error}</p>}
    <label className="flex gap-3 items-center">Currency
      <select value={currency || ""} onChange={(event) => onChange({ salaryMin, salaryMax, interval, currency: event.target.value || null, includeUnknown })} className="border bg-[var(--color-paper)] p-2">
        <option value="">Any currency</option>{["USD", "CAD", "EUR", "GBP"].map((code) => <option key={code}>{code}</option>)}
      </select>
    </label>
    <label className="flex gap-3 items-start"><input type="checkbox" checked={includeUnknown} onChange={(event) => onChange({ salaryMin, salaryMax, currency, interval, includeUnknown: event.target.checked })} />Include unknown pay bounds (a selected unit and currency must still be known)</label>
    <button type="button" className="border px-3 py-2 self-start" onClick={() => { setMinText(""); setMaxText(""); setError(""); onChange({ salaryMin: null, salaryMax: null, currency: null, interval: null, includeUnknown: false }); }}>Clear compensation filter</button>
  </fieldset>;
}
