import { useEffect, useId, useState } from "react";
import type { ExperienceMatchMode } from "../../bindings/ExperienceMatchMode";

interface ExperienceValues {
  experienceMin: number | null;
  experienceMax: number | null;
  experienceMatchMode: ExperienceMatchMode;
  newGrad: string | null;
  includeUnknown: boolean;
}

export interface ExperienceFilterProps extends ExperienceValues {
  onChange: (values: ExperienceValues) => void;
  onDraftChange?: () => void;
  onValidityChange?: (valid: boolean) => void;
}

const PRESETS = [
  { label: "Any", min: null, max: null },
  { label: "0–1", min: 0, max: 1 },
  { label: "0–2", min: 0, max: 2 },
  { label: "3–5", min: 3, max: 5 },
  { label: "5+", min: 5, max: null },
];
const NEW_GRAD_OPTIONS = [
  { id: "any", label: "Any New Grad status" },
  { id: "only", label: "New Grad only" },
  { id: "exclude", label: "Exclude New Grad" },
];
const buttonClass = "px-2 py-1 text-xs border border-[var(--color-ink)] cursor-pointer aria-pressed:bg-[var(--color-ink)] aria-pressed:text-[var(--color-paper)]";
const validBound = (value: string) => value === "" || (/^\d+$/.test(value) && Number.isSafeInteger(Number(value)));

export function ExperienceFilter({ experienceMin, experienceMax, experienceMatchMode, newGrad, includeUnknown, onChange, onValidityChange, onDraftChange }: ExperienceFilterProps) {
  const [minText, setMinText] = useState(String(experienceMin ?? ""));
  const [maxText, setMaxText] = useState(String(experienceMax ?? ""));
  const errorId = useId();
  useEffect(() => { setMinText(String(experienceMin ?? "")); }, [experienceMin]);
  useEffect(() => { setMaxText(String(experienceMax ?? "")); }, [experienceMax]);
  const minValid = validBound(minText);
  const maxValid = validBound(maxText);
  const ordered = minText === "" || maxText === "" || Number(minText) <= Number(maxText);
  const valid = minValid && maxValid && ordered;
  useEffect(() => { onValidityChange?.(valid); }, [valid, onValidityChange]);
  const active = experienceMin != null || experienceMax != null;
  const overlap = active && experienceMatchMode === "advertised_range_overlap";
  const currentMin = minValid && minText !== "" ? Number(minText) : experienceMin;
  const currentMax = maxValid && maxText !== "" ? Number(maxText) : experienceMax;
  const values = { experienceMin: currentMin, experienceMax: currentMax, experienceMatchMode, newGrad: newGrad || "any", includeUnknown };

  function setBounds(min: number | null, max: number | null) {
    const nextActive = min != null || max != null;
    onChange({ ...values, experienceMin: min, experienceMax: max,
      experienceMatchMode: active && nextActive ? experienceMatchMode : "required_minimum",
      includeUnknown: active && nextActive ? includeUnknown : false });
  }
  function editBound(which: "min" | "max", text: string) {
    onDraftChange?.();
    const nextMin = which === "min" ? text : minText;
    const nextMax = which === "max" ? text : maxText;
    if (which === "min") setMinText(text); else setMaxText(text);
    if (validBound(nextMin) && validBound(nextMax) && (nextMin === "" || nextMax === "" || Number(nextMin) <= Number(nextMax))) {
      setBounds(nextMin === "" ? null : Number(nextMin), nextMax === "" ? null : Number(nextMax));
    }
  }
  function preset(min: number | null, max: number | null) {
    setMinText(String(min ?? "")); setMaxText(String(max ?? ""));
    setBounds(min, max);
  }

  return (
    <div className="flex flex-col gap-3 font-mono text-xs">
      <div className="flex flex-wrap justify-between gap-2">
        <span className="font-bold">{overlap ? "Advertised range overlap (saved rule)" : "Minimum required experience"}</span>
        {overlap && <button type="button" className={buttonClass} onClick={() => onChange({ ...values, experienceMatchMode: "required_minimum" })}>Use minimum required experience</button>}
      </div>
      <div role="group" aria-label="Years of experience" className="flex flex-wrap gap-1.5">
        {PRESETS.map(({ label, min, max }) => <button key={label} type="button" className={buttonClass} aria-pressed={valid && experienceMin === min && experienceMax === max} onClick={() => preset(min, max)}>{label}</button>)}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <span>Custom:</span>
        <label>Min <input aria-label="Minimum experience (years)" aria-invalid={!minValid || !ordered} aria-describedby={!valid ? errorId : undefined} type="text" inputMode="numeric" value={minText} onChange={(event) => editBound("min", event.target.value)} className="w-16 border border-[var(--color-ink)] bg-[var(--color-paper)] p-1" /></label>
        <label>Max <input aria-label="Maximum experience (years)" aria-invalid={!maxValid || !ordered} aria-describedby={!valid ? errorId : undefined} type="text" inputMode="numeric" value={maxText} onChange={(event) => editBound("max", event.target.value)} className="w-16 border border-[var(--color-ink)] bg-[var(--color-paper)] p-1" /></label>
        <span>years, inclusive</span>
      </div>
      {!valid && <p id={errorId} role="alert" className="text-[var(--color-danger)]">Enter non-negative whole years, with minimum no greater than maximum.</p>}
      <label className="flex items-center gap-2"><input type="checkbox" checked={includeUnknown} onChange={(event) => onChange({ ...values, includeUnknown: event.target.checked })} />{overlap ? "Include jobs without stated experience" : "Include jobs with unknown required experience"}</label>
      <div role="group" aria-label="New Grad eligibility" className="flex flex-wrap gap-1.5">
        {NEW_GRAD_OPTIONS.map(({ id, label }) => <button key={id} type="button" className={buttonClass} aria-pressed={(newGrad || "any") === id} onClick={() => onChange({ ...values, newGrad: id })}>{label}</button>)}
      </div>
      <p className="text-[var(--color-ink-mute)]">New Grad eligibility is separate from numeric experience and must also match.</p>
    </div>
  );
}
