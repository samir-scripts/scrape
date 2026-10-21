import { useEffect, useState } from "react";
import catalogData from "../../../generated/workflowTemplates.json";
import type { WorkflowTemplateCatalog } from "../../../bindings/WorkflowTemplateCatalog";
import type { WorkflowSourceSpec } from "../../../bindings/WorkflowSourceSpec";
import type { CompanyLogo as LogoReport } from "../../../bindings/CompanyLogo";
import { normalizedEndpoint } from "../../../lib/yamlWorkflow";
import { CompanyLogo } from "../../ui/CompanyLogo";
import { SearchCombobox } from "../../ui/Select";
import { Button } from "../../ui/Button";

const catalog = catalogData as unknown as WorkflowTemplateCatalog;
export const companyOptions = catalog.companies.map(company => {
  const template = catalog.templates.find(template => template.id === company.template_id)!;
  const source = template.spec.sources.find(source => source.id === company.source_id)!;
  let domain = source.extractor.url;
  try { domain = new URL(source.extractor.url).hostname; } catch { /* Board identifiers remain visible. */ }
  return { company, template, source, domain };
});
export const COMPANY_QUICK_PRESETS = [
  { id: "cloudflare", label: "Cloudflare" },
  { id: "linear", label: "Linear" },
  { id: "bitwarden", label: "Bitwarden" },
  { id: "sep", label: "SEP" },
];
export function selectedCompanyId(source: WorkflowSourceSpec): string | null {
  const endpoint = normalizedEndpoint(source.extractor.kind, source.extractor.url);
  return companyOptions.find(option => normalizedEndpoint(option.source.extractor.kind, option.source.extractor.url) === endpoint)?.company.id ?? null;
}
export function CompanyPicker({ value, onChange, onCustom, disabled, logo }: {
  value: string | null; onChange: (companyId: string) => void; onCustom: () => void;
  disabled?: boolean; logo?: LogoReport | null;
}) {
  const selected = companyOptions.find(option => option.company.id === value);
  const [query, setQuery] = useState(selected?.company.name ?? "");
  useEffect(() => { setQuery(selected?.company.name ?? ""); }, [value]);
  const restore = () => setQuery(selected?.company.name ?? "");
  return <div className="grid min-w-0 gap-2">
    <div className="flex flex-wrap items-center justify-between gap-2">
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        <span className="text-[var(--color-text-muted)] font-semibold uppercase">Popular:</span>
        {COMPANY_QUICK_PRESETS.map(preset => (
          <button
            key={preset.id}
            type="button"
            disabled={disabled}
            aria-pressed={value === preset.id}
            className={`min-h-7 px-2 border text-xs font-semibold uppercase transition-colors ${value === preset.id ? "bg-[var(--color-text)] text-[var(--color-surface)] border-[var(--color-border)]" : "border-[var(--color-border)] hover:bg-[var(--color-surface-raised)]"}`}
            onClick={() => onChange(preset.id)}
          >
            {preset.label}
          </button>
        ))}
      </div>
      <Button type="button" size="sm" disabled={disabled} onClick={onCustom}>Enter a custom careers URL</Button>
    </div>
    <SearchCombobox aria-label="Search companies" value={value ?? ""} query={query}
      placeholder="Search company, provider or board domain" disabled={disabled}
      options={companyOptions.map(({company, source, domain, template}) => ({ value: company.id, label: company.name,
        searchText: `${company.name} ${source.extractor.kind} ${domain} ${template.readiness}` }))}
      onQueryChange={setQuery} onBlur={restore} onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); restore(); } }}
      onChange={id => { if (id) { onChange(id); setQuery(companyOptions.find(option => option.company.id === id)?.company.name ?? ""); } }}
      renderOption={option => {
        const item = companyOptions.find(item => item.company.id === option.value)!;
        return <span className="flex min-w-0 items-center gap-2 normal-case font-sans">
          <CompanyLogo company={item.company.name} logo={item.company.logo} size="sm" />
          <span className="min-w-0 break-words"><span className="font-semibold">{item.company.name}</span><span className="block text-[var(--color-ink-mute)]">{item.source.extractor.kind} · {item.template.readiness.replaceAll("_", " ")}</span></span>
        </span>;
      }} />
    {selected && <div className="flex min-w-0 items-start gap-3 border border-[var(--color-border)] p-3 bg-[var(--color-surface-raised)]" aria-label="Selected company details">
      <CompanyLogo company={selected.company.name} logo={logo ?? selected.company.logo} size="md" />
      <div className="min-w-0 flex-1 grid gap-1">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="font-bold text-sm">{selected.company.name}</p>
          <span className="px-1.5 py-0.5 border border-[var(--color-border)] text-[10px] font-semibold uppercase bg-[var(--color-surface)]">
            {selected.source.extractor.kind} · {selected.template.readiness.replaceAll("_", " ")}
          </span>
        </div>
        <p className="text-xs text-[var(--color-text-muted)] break-all">{selected.domain} · {selected.source.extractor.url}</p>
        {!selected.source.enabled && <p className="text-xs text-[var(--color-warning)] font-semibold">Source disabled until explicitly enabled</p>}
        <p className="text-xs text-[var(--color-text-muted)]">{selected.template.description}</p>
        <p className="text-[11px] text-[var(--color-text-muted)]">Logo: {(logo ?? selected.company.logo)?.diagnostic || (logo ?? selected.company.logo)?.status?.replaceAll("_", " ") || "Initials fallback until verified artwork"}</p>
      </div>
    </div>}
  </div>;
}
