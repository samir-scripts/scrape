import { useEffect, useId, useState } from "react";
import type { PostingRules } from "../../../bindings/PostingRules";
import { COUNTRIES, REGIONS_BY_COUNTRY } from "../../../lib/locationData";
import { Input } from "../../ui/Input";
import { Icon } from "../../ui/Icon";

type Choice = { code: string; name: string };
function SearchChoice({ label, value, choices, disabled, placeholder, resetAfterSelect, onSelect, onDraftChange }: {
  label: string; value: string; choices: Choice[]; disabled?: boolean; placeholder: string; resetAfterSelect?: boolean;
  onSelect: (code: string) => void; onDraftChange: () => void;
}) {
  const id = useId();
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const fold = (value: string) => value.normalize("NFKD").replace(/[\u0300-\u036f]/g, "").trim().toLowerCase();
  const find = (query: string) => {
    const exact = choices.filter(c => fold(c.code) === fold(query) || fold(c.name) === fold(query));
    if (exact.length === 1) return exact[0];
    const short = choices.filter(c => c.code.includes("-") && fold(c.code.split("-")[1]) === fold(query));
    return short.length === 1 ? short[0] : undefined;
  };
  const invalid = !!text.trim() && text !== value && !find(text);
  const commit = () => {
    if (!text.trim()) { if (!resetAfterSelect && value) onSelect(""); return; }
    if (text === value) return;
    const choice = find(text);
    if (choice) { onSelect(choice.code); setText(resetAfterSelect ? "" : choice.name); }
  };
  return <div className="grid gap-1 min-w-0">
    <label htmlFor={id}>{label}</label>
    <Input id={id} aria-label={label} list={`${id}-choices`} autoComplete="off" value={text} disabled={disabled} placeholder={placeholder}
      aria-invalid={invalid} aria-describedby={invalid ? `${id}-error` : undefined}
      onChange={event => { setText(event.target.value); onDraftChange(); }} onBlur={commit}
      onKeyDown={event => { if (event.key === "Enter") { event.preventDefault(); commit(); } if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); setText(value); } }} />
    <datalist id={`${id}-choices`}>{choices.map(choice => <option key={choice.code} value={choice.name}>{choice.code}</option>)}</datalist>
    {invalid && <p id={`${id}-error`} className="text-[var(--color-danger)]">Choose a listed {label.toLowerCase()} by name or code.</p>}
  </div>;
}
export function GeographyMatching({ rules, onChange, onDraftChange }: { rules: PostingRules; onChange: (patch: Partial<PostingRules>) => void; onDraftChange: () => void }) {
  const countries = rules.countries.map(code => code.toUpperCase());
  const regions = countries.flatMap(code => (REGIONS_BY_COUNTRY[code] || []).map(region => ({ code: `${code}-${region.code}`, name: countries.length > 1 ? `${region.name} (${code})` : region.name })));
  const countryName = (code: string) => COUNTRIES.find(c => c.code === code)?.name || code;
  const value = countries.length === 1 ? countryName(countries[0]) : countries.length ? `Multiple countries: ${countries.join(", ")}` : "";
  const canonical = (region: string) => region.includes("-") ? region.toUpperCase() : countries.length === 1 ? `${countries[0]}-${region.toUpperCase()}` : region.toUpperCase();
  return <fieldset className="grid gap-3 text-sm border-t pt-3">
    <legend className="font-bold">Location and workplace</legend>
    <SearchChoice label="Country" value={value} choices={COUNTRIES} placeholder="Any country — search name or code" onDraftChange={onDraftChange}
      onSelect={code => onChange({ countries: code ? [code] : [], regions: code ? rules.regions.map(canonical).filter(region => region.startsWith(`${code}-`)) : [] })} />
    <SearchChoice label="State/province" value="" choices={regions} disabled={!regions.length} placeholder={countries.length ? (regions.length ? "Any state — search to add" : "No subdivision list available") : "Choose a country first"} resetAfterSelect onDraftChange={onDraftChange}
      onSelect={code => { if (!rules.regions.map(canonical).includes(code)) onChange({ regions: [...rules.regions, code] }); }} />
    {!!rules.regions.length && <div className="flex gap-2 flex-wrap">{rules.regions.map(region => <button key={region} type="button" className="border min-h-8 px-2 py-1 inline-flex gap-2 items-center" aria-label={`Remove ${regions.find(r => r.code === canonical(region))?.name || region}`} onClick={() => onChange({ regions: rules.regions.filter(value => value !== region) })}>
      {regions.find(r => r.code === canonical(region))?.name || region}<Icon name="close" />
    </button>)}</div>}
    <p className="text-[var(--color-ink-mute)]">Country, state and workplace apply together. Remote alone does not imply a country. Leave a field empty for no restriction.</p>
    <fieldset className="flex gap-4 flex-wrap"><legend className="mb-1">Workplace — match any selected</legend>
      {([['remote','Remote'],['hybrid','Hybrid'],['onsite','In-person / On-site']] as const).map(([code,label]) => <label key={code} className="flex gap-2 items-center min-h-8"><input type="checkbox" checked={rules.workplaces.includes(code)} onChange={event => onChange({ workplaces: event.target.checked ? [...rules.workplaces,code] : rules.workplaces.filter(value => value !== code) })} />{label}</label>)}
    </fieldset>
    <p className="text-[var(--color-ink-mute)]">In-person, in-office and on-site use the same category.</p>
    <label className="flex items-start gap-2"><input type="checkbox" checked={rules.location_include_unknown} onChange={event => onChange({ location_include_unknown: event.target.checked })} />Include postings with unknown location</label>
  </fieldset>;
}
