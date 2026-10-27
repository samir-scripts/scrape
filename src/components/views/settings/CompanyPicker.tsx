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
  const source = company.source;
  let domain = source.extractor.url;
  try { domain = new URL(source.extractor.url).hostname; } catch { /* Board identifiers remain visible. */ }
  return { company, source, domain };
});
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
    <div className="flex justify-end">
      <Button data-control-action="workflow.company_custom_url" type="button" size="sm" disabled={disabled} onClick={onCustom}>Enter a custom careers URL</Button>
    </div>
    <SearchCombobox actionId="workflow.company_search" aria-label="Search companies" value={value ?? ""} query={query}
      placeholder="Search company, provider or board domain" disabled={disabled}
      options={companyOptions.map(({company, source, domain}) => ({ value: company.id, label: company.name,
        searchText: `${company.name} ${source.extractor.kind} ${domain} ${company.readiness}` }))}
      onQueryChange={setQuery} onBlur={restore} onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); restore(); } }}
      onChange={id => { if (id) { onChange(id); setQuery(companyOptions.find(option => option.company.id === id)?.company.name ?? ""); } }}
      renderOption={option => {
        const item = companyOptions.find(item => item.company.id === option.value)!;
        return <span className="flex min-w-0 items-center gap-2 normal-case font-sans">
          <CompanyLogo company={item.company.name} logo={item.company.logo} size="sm" />
          <span className="min-w-0 break-words"><span className="font-semibold">{item.company.name}</span><span className="block text-[var(--color-ink-mute)]">{item.source.extractor.kind} · {item.company.readiness.replaceAll("_", " ")}</span></span>
        </span>;
      }} />
    {selected && <div className="flex min-w-0 items-start gap-3 border border-[var(--color-border)] p-3 bg-[var(--color-surface-raised)]" aria-label="Selected company details">
      <CompanyLogo company={selected.company.name} logo={logo ?? selected.company.logo} size="md" />
      <div className="min-w-0 flex-1 grid gap-1">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="font-bold text-sm">{selected.company.name}</p>
          <span className="px-1.5 py-0.5 border border-[var(--color-border)] text-[10px] font-semibold uppercase bg-[var(--color-surface)]">
            {selected.source.extractor.kind} · {selected.company.readiness.replaceAll("_", " ")}
          </span>
        </div>
        <p className="text-xs text-[var(--color-text-muted)] break-all">{selected.domain} · {selected.source.extractor.url}</p>
        {!selected.source.enabled && <p className="text-xs text-[var(--color-warning)] font-semibold">Source disabled until explicitly enabled</p>}
        <p className="text-xs text-[var(--color-text-muted)]">{selected.company.description}</p>
        <p className="text-[11px] text-[var(--color-text-muted)]">Logo: {(logo ?? selected.company.logo)?.diagnostic || (logo ?? selected.company.logo)?.status?.replaceAll("_", " ") || "Initials fallback until verified artwork"}</p>
      </div>
    </div>}
  </div>;
}
