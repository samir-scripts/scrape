import { useEffect, useMemo, useState } from "react";
import { CompatibilityFilters } from "./CompatibilityFilters";
import { CompanyPicker, selectedCompanyId, companyOptions } from "./CompanyPicker";
import { ExperienceFilter } from "../../filters/ExperienceFilter";
import type { WorkflowPreview } from "../../../bindings/WorkflowPreview";
import type { WorkflowDraftChange } from "../../../hooks/useWorkflowDraft";
import { KeywordMatching } from "./KeywordMatching";
import { GeographyMatching } from "./GeographyMatching";
import { INGESTION_LIMITS } from "../../../bindings/ingestionLimits";
import { SalaryRangeFilter } from "../../filters/SalaryRangeFilter";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowSourceSpec } from "../../../bindings/WorkflowSourceSpec";
import type { PostingRules } from "../../../bindings/PostingRules";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
import { JobLimitInput } from "./JobLimitInput";
import { CustomExtractorConfig } from "./CustomExtractorConfig";
import type { Source } from "../../../bindings/Source";
import { MultiSelectCombobox } from "../../ui/MultiSelectCombobox";
import type { MultiSelectOption } from "../../ui/MultiSelectCombobox";
import { api } from "../../../lib/api";
import { normalizedEndpoint } from "../../../lib/yamlWorkflow";
import { ClockPicker } from "../../ui/ClockPicker";
import { from24Hour, pad2, to24Hour } from "../../../lib/cronClock";

const loadRoleOptions = async (query: string): Promise<MultiSelectOption[]> =>
  (await api.searchRoleTitles(query)).map(({ title, soc_code, priority_tier }) => ({
    value: title, label: title,
    description: `${soc_code} · ${["Canonical title", "Reported title", "Alternate title"][priority_tier - 1]}`,
  }));
const companySelectionOptions: MultiSelectOption[] = companyOptions.map(({ company, source, domain }) => ({
  value: company.name, label: company.name,
  searchText: `${company.name} ${source.extractor.kind} ${domain} ${company.readiness}`,
  description: `${source.extractor.kind} · ${company.readiness.replaceAll("_", " ")}`,
}));

const loadCompanyOptions = async (query: string): Promise<MultiSelectOption[]> => {
  const suggestions = await api.searchCompanies(query);
  if (suggestions.length) {
    return suggestions.map(suggestion => {
      const preset = companyOptions.find(option => option.company.id === suggestion.catalog_company_id);
      const seen = suggestion.jobs_seen
        ? `${suggestion.jobs_seen} saved posting${suggestion.jobs_seen === 1 ? "" : "s"}`
        : "";
      return {
        value: suggestion.name,
        label: suggestion.name,
        searchText: preset
          ? `${suggestion.name} ${preset.source.extractor.kind} ${preset.domain} ${preset.company.readiness}`
          : suggestion.name,
        description: preset
          ? `${preset.source.extractor.kind} · ${preset.company.readiness.replaceAll("_", " ")}${seen ? ` · ${seen}` : ""}`
          : `${seen} · employer filter only; no verified career source`,
      };
    });
  }
  const tokens = query.toLocaleLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  return companySelectionOptions.filter(option => {
    const words = (option.searchText ?? option.label).toLocaleLowerCase().split(/[^\p{L}\p{N}]+/u);
    return tokens.every(token => words.some(word => word.startsWith(token)));
  });
};

export const emptyRules = (): PostingRules => ({
  target_roles: [],
  companies: [],
  keywords: null,
  posting_date: null,
  compensation: null,
  role_types: [],
  workplaces: [],
  experience_min: null,
  experience_max: null,
  experience_include_unknown: false,
  experience_match_mode: "required_minimum",
  new_grad: null,
  sponsorship: [],
  work_auth: [],
  clearance: [],
  countries: [],
  regions: [],
  cities: [],
  location_include_unknown: false,
});
export const words = (text: string) =>
  text
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
export function WorkflowSections({
  draft,
  change,
  advanced,
  section,
  onSourceBlur,
  initialSource,
  onTrustChanged,
  selectCompany,
  setTargetCompanies,
  attachCompanyBoard,
  onDetect,
  preview,
  is24h = false,
}: {
  onSourceBlur?: () => void;
  draft: WorkflowSpec;
  change: WorkflowDraftChange;
  advanced: boolean;
  section: "Roles" | "Criteria" | "Sources" | "Schedule" | "Retention";
  initialSource?: Source | null;
  onTrustChanged?: () => void;
  selectCompany?: (sourceId: string, companyId: string) => void;
  setTargetCompanies: (companies: string[]) => void;
  attachCompanyBoard: (companyId: string) => void;
  onDetect?: (sourceIndex: number) => void;
  preview?: WorkflowPreview | null;
  is24h?: boolean;
}) {
  const filters = draft.filters || {};
  const rules = { ...emptyRules(), ...filters.matching };
  const compensationMatchMode = rules.compensation?.match_mode ?? "range_overlap";
  const [adzunaCredentialConfigured, setAdzunaCredentialConfigured] = useState<boolean | null>(null);
  useEffect(() => {
    void api.getAdzunaCredentialStatus()
      .then(({ configured }) => setAdzunaCredentialConfigured(configured))
      .catch(() => setAdzunaCredentialConfigured(false));
  }, []);
  const setRules = (patch: Partial<PostingRules>) =>
    change(current => ({
      ...current,
      filters: { ...current.filters, matching: { ...emptyRules(), ...current.filters?.matching, ...patch } },
    }));
  const schedule = draft.schedule || {};
  const clockValue = useMemo(() => {
    const timeMatch = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(schedule.time || "09:00");
    const clockTime = from24Hour(Number(timeMatch?.[1] ?? 9));
    return { ...clockTime, minute: Number(timeMatch?.[2] ?? 0), cadence: "daily" as const };
  }, [schedule.time]);
  const setSchedule = (patch: Partial<NonNullable<WorkflowSpec["schedule"]>>) =>
    change(current => ({ ...current, schedule: { ...current.schedule, ...patch } }));
  const field = (
    label: string,
    value: string | number,
    set: (value: string) => void,
    type = "text",
  ) => (
    <label key={label} className="flex flex-col gap-1 text-sm">
      {label}
      <Input
        aria-label={label}
        type={type}
        step={type === "number" ? "any" : undefined}
        value={value}
        onChange={(e) => set(e.target.value)}
      />
    </label>
  );
  if (section === "Roles") {
    return (
      <div className="grid gap-4">
        <section aria-label="Workflow identification" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <h4 className="font-bold text-sm">Workflow Name</h4>
          {field(
            "Workflow name",
            draft.metadata?.name || "",
            (name) => change(current => ({ ...current, metadata: { ...current.metadata, name } }))
          )}
          <p className="text-xs text-[var(--color-ink-mute)]">
            Auto-named from your target roles and companies if left empty.
          </p>
        </section>

        <section aria-label="Target roles" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <h4 className="font-bold text-sm">Target Roles</h4>
          <MultiSelectCombobox label="Target roles" value={rules.target_roles || []}
            options={[]} loadOptions={loadRoleOptions} allowCustom onChange={target_roles => setRules({ target_roles })} />
          <p className="text-xs text-[var(--color-ink-mute)]">
            Job titles must contain any selected text; matching is literal, not every title in an occupation. Choose alternate titles or add custom text as needed. Leave empty to match any role.
          </p>
          <details className="text-xs text-[var(--color-ink-mute)]">
            <summary className="cursor-pointer underline">Role-title sources & attribution · O*NET 31.0</summary>
            <p className="mt-2">
              <a className="underline" href="https://www.onetcenter.org/database.html" target="_blank" rel="noreferrer">O*NET 31.0 database</a> · <a className="underline" href="https://creativecommons.org/licenses/by/4.0/" target="_blank" rel="noreferrer">CC BY 4.0 license</a>. Suggestions combine canonical, reported and alternate titles; matching remains literal. Includes information from the O*NET 31.0 Database by USDOL/ETA. Scrape's derived catalog is not endorsed or tested by USDOL/ETA.
            </p>
          </details>
        </section>

        <section aria-label="Target companies" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <h4 className="font-bold text-sm">Target Companies (Optional)</h4>
          <MultiSelectCombobox label="Companies (Optional)" value={rules.companies || []}
            options={[]} loadOptions={loadCompanyOptions} allowCustom describedBy="company-coverage-help" onChange={setTargetCompanies} />
          <p id="company-coverage-help" className="text-xs text-[var(--color-ink-mute)]">
            Available career boards attach automatically when selected. Removing a company filter leaves its sources in Sources. Custom company filters restrict matching; public-feed coverage is not guaranteed.
          </p>
        </section>


        <section aria-label="Job keywords" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <h4 className="font-bold text-sm">Keywords</h4>
          <KeywordMatching rules={rules} change={change} />
        </section>
      </div>
    );
  }
  if (section === "Sources") {
    const endpointMap = new Map<string, number>();
    (draft.sources || []).forEach((s) => {
      if (s.extractor?.kind === "search" || s.extractor?.url?.trim()) {
        const ep = normalizedEndpoint(s.extractor.kind, s.extractor.url);
        endpointMap.set(ep, (endpointMap.get(ep) || 0) + 1);
      }
    });
    const hasDuplicates = Array.from(endpointMap.values()).some((c) => c > 1);
    const allDisabled = (draft.sources?.length ?? 0) > 0 && !draft.sources.some((s) => s.enabled);

    return (
      <div className="grid gap-4">
        <div className="border border-[var(--color-border)] p-4 bg-[var(--color-surface)] grid gap-2">
          <div className="flex items-center justify-between">
            <span className="font-bold text-sm">✓ Cross-Company Search</span>
            <span className="text-xs px-2 py-0.5 bg-[var(--color-text)] text-[var(--color-surface)] font-semibold uppercase">
              {(draft.sources || []).some(s => s.extractor.kind === "search" && s.enabled) ? "Active" : "Available"}
            </span>
          </div>
          <p className="text-xs text-[var(--color-ink-mute)]">
            The Muse is a bounded public feed; Arbeitnow is Germany-focused and may lack original dates. Jobicy is remote-only, covers its seven-day feed after a three-hour delay, and synchronizes at most hourly. Adzuna requires personal API credentials configured in Settings and explicit source enablement. Provider coverage is limited, not every employer; Google, Apple, Netflix and Amazon scripts remain separately review-gated.
          </p>
        </div>

        {rules.companies?.map(targetCo => {
          const match = companyOptions.find(option => option.company.name.toLocaleLowerCase() === targetCo.trim().toLocaleLowerCase());
          if (!match) return null;
          const endpoint = normalizedEndpoint(match.source.extractor.kind, match.source.extractor.url);
          const alreadyAttached = (draft.sources || []).some(source => normalizedEndpoint(source.extractor.kind, source.extractor.url) === endpoint);
          if (alreadyAttached) return null;
          return <div key={targetCo} className="border border-[var(--color-border)] p-3 bg-[var(--color-surface-raised)] flex items-center justify-between gap-3 text-xs">
            <span>Career board for <strong>{match.company.name}</strong> · {match.company.readiness.replaceAll("_", " ")}. Setup-required, review and script sources attach disabled.</span>
            <Button type="button" size="sm" aria-label={`Attach career board for ${match.company.name}`}
              onClick={() => attachCompanyBoard(match.company.id)}>Attach career board</Button>
          </div>;
        })}
        <p className="text-xs text-[var(--color-text-muted)] max-w-prose">
          New workflows use selected feeds and verified direct boards within bounded request limits; coverage is never every employer. Jobicy has a seven-day publication window and three-hour delay. Adzuna requires local credentials in Settings &gt; System &gt; Job providers and explicit enablement. Preview applies all matching locally. Google, Apple, Netflix and Amazon remain available only as separately review-gated company sources; USAJOBS is not supported.
        </p>
        <div className="flex items-center justify-between gap-2 pt-2 border-t border-[var(--color-border)]">
          <h4 className="font-bold text-sm">
            Sources ({draft.sources?.length ?? 0})
          </h4>
          <Button
            type="button"
            size="sm"
            aria-label="Add source"
            onClick={() => {
              const newId = `source-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
              const count = (draft.sources?.length ?? 0) + 1;
              const newSource: WorkflowSourceSpec = {
                id: newId,
                label: `Source ${count}`,
                enabled: true,
                extractor: { kind: "jsonld", url: "" },
              };
              change({
                ...draft,
                sources: [...(draft.sources ?? []), newSource],
              });
            }}
          >
            + Add source
          </Button>
        </div>
        <div className="border-t border-[var(--color-border)] pt-2">
          <JobLimitInput
            value={draft.max_jobs_per_run ?? INGESTION_LIMITS.default}
            onChange={value => change(value == null ? { ...draft } : { ...draft, max_jobs_per_run: value })}
          />
        </div>

        {hasDuplicates && (
          <div role="alert" className="border border-[var(--color-danger)] p-2 text-xs text-[var(--color-danger)] bg-[var(--color-paper)]">
            Duplicate source endpoint detected. Each source in this workflow must have a distinct normalized endpoint.
          </div>
        )}
        {allDisabled && (
          <div role="alert" className="border border-[var(--color-warning)] p-2 text-xs text-[var(--color-warning)] bg-[var(--color-paper)]">
            At least one source must be enabled for this workflow to run.
          </div>
        )}

        <div className="grid gap-3">
          {(draft.sources || []).map((source, index) => (
            <div
              key={source.id}
              data-source-index={index}
              className="border border-[var(--color-border)] p-3 grid gap-3 bg-[var(--color-surface)]"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border)] pb-2">
                <div className="flex items-center gap-2">
                  <label className="flex items-center gap-1.5 text-xs font-semibold uppercase">
                    <input
                      type="checkbox"
                      aria-label={`Enable ${source.label || `Source ${index + 1}`}`}
                      checked={source.enabled}
                      onChange={(e) => {
                        const nextSources = [...draft.sources];
                        nextSources[index] = { ...source, enabled: e.target.checked };
                        change({ ...draft, sources: nextSources });
                      }}
                    />
                    Enabled
                  </label>
                  <span className="px-1.5 py-0.5 border border-[var(--color-border)] text-xs uppercase bg-[var(--color-paper)]">
                    {source.extractor.kind}
                  </span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    type="button"
                    size="sm"
                    aria-label={`Move source ${source.label || index + 1} up`}
                    disabled={index === 0}
                    onClick={() => {
                      if (index === 0) return;
                      const nextSources = [...draft.sources];
                      const [moved] = nextSources.splice(index, 1);
                      nextSources.splice(index - 1, 0, moved);
                      change({ ...draft, sources: nextSources });
                    }}
                  >
                    ↑
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    aria-label={`Move source ${source.label || index + 1} down`}
                    disabled={index === draft.sources.length - 1}
                    onClick={() => {
                      if (index === draft.sources.length - 1) return;
                      const nextSources = [...draft.sources];
                      const [moved] = nextSources.splice(index, 1);
                      nextSources.splice(index + 1, 0, moved);
                      change({ ...draft, sources: nextSources });
                    }}
                  >
                    ↓
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="danger"
                    aria-label={`Remove source ${source.label || index + 1}`}
                    disabled={draft.sources.length <= 1}
                    onClick={() => {
                      if (draft.sources.length <= 1) return;
                      const nextSources = draft.sources.filter((_, i) => i !== index);
                      change({ ...draft, sources: nextSources });
                    }}
                  >
                    Remove
                  </Button>
                </div>
              </div>
              {source.extractor.kind !== "search" && <CompanyPicker value={selectedCompanyId(source)}
                onChange={companyId => selectCompany?.(source.id, companyId)}
                onCustom={() => {}}
                logo={preview?.branding.find(item => item.source_id === source.id && item.company === source.employer_hint)?.logo} />}

              <div className="grid gap-3">
                {source.extractor.kind !== "search" && <div className="flex flex-col gap-1 text-sm">
                  Source URL or board identifier
                  <div className="flex gap-2">
                    <Input
                      aria-label={index === 0 ? "Source URL or board identifier" : `Source ${index + 1} URL or board identifier`}
                      value={source.extractor.url}
                      placeholder="https://boards.greenhouse.io/company"
                      onBlur={() => {
                        if (index === 0 && onSourceBlur) onSourceBlur();
                      }}
                      onChange={(e) => {
                        const nextSources = [...draft.sources];
                        nextSources[index] = {
                          ...source,
                          extractor: { ...source.extractor, url: e.target.value },
                        };
                        change({ ...draft, sources: nextSources });
                      }}
                    />
                    <Button
                      type="button"
                      size="sm"
                      tabIndex={-1}
                      aria-label={`Detect source ${index + 1}`}
                      onClick={() => onDetect?.(index)}
                    >
                      Detect
                    </Button>
                  </div>
                </div>}
                {source.extractor.kind === "search" && <label className="grid gap-1 text-sm">Public listings provider
                  <Select aria-label={`Source ${index + 1} search provider`}
                    value={!source.extractor.url || source.extractor.url === "default" ? "themuse" : source.extractor.url}
                    options={[
                      { value: "themuse", label: "The Muse — bounded worldwide listings" },
                      { value: "arbeitnow", label: "Arbeitnow — Germany-focused feed" },
                      { value: "jobicy", label: "Jobicy — remote feed, seven days, hourly cache" },
                      { value: "adzuna", label: "Adzuna — US listings (requires credentials)" },
                    ]}
                    onChange={provider => change(current => ({ ...current, sources: current.sources.map(item => item.id === source.id ? { ...item, extractor: { ...item.extractor, url: provider, source_config: {} } } : item) }))} />
                  {source.extractor.url === "adzuna" && (
                    <span role="status" className="text-xs text-[var(--color-ink-mute)]">
                      {adzunaCredentialConfigured
                        ? "Credentials configured; enable this source explicitly to search."
                        : "Setup required: configure credentials in Settings > System > Job providers. Saving credentials does not enable this source."}
                    </span>
                  )}
                </label>}

                <details className="border-t border-[var(--color-border)] pt-2 text-xs">
                  <summary className="cursor-pointer font-semibold uppercase text-[var(--color-text-muted)] hover:text-[var(--color-text)] py-1">
                    Customize source labels and metadata
                  </summary>
                  <div className="grid gap-3 pt-2">
                    <label className="flex flex-col gap-1 text-sm">
                      Source label
                      <Input
                        aria-label={index === 0 ? "Source label" : `Source ${index + 1} label`}
                        value={source.label}
                        placeholder="e.g. Cloudflare Greenhouse"
                        onChange={(e) => {
                          const nextSources = [...draft.sources];
                          nextSources[index] = { ...source, label: e.target.value };
                          change({ ...draft, sources: nextSources });
                        }}
                      />
                    </label>

                    <label className="flex flex-col gap-1 text-sm">
                      Employer hint (used for branding if unlabelled)
                      <Input
                        aria-label={`Source ${index + 1} employer hint`}
                        value={source.employer_hint || ""}
                        placeholder="e.g. Acme Corp"
                        onChange={(e) => {
                          const nextSources = [...draft.sources];
                          nextSources[index] = {
                            ...source,
                            employer_hint: e.target.value || undefined,
                          };
                          change({ ...draft, sources: nextSources });
                        }}
                      />
                    </label>

                    {index === 0 && field(
                      "Company hint (optional)",
                      draft.metadata.company || "",
                      (company) =>
                        change({
                          ...draft,
                          metadata: {
                            ...draft.metadata,
                            company: company || undefined,
                          },
                        }),
                    )}
                  </div>
                </details>
                {advanced && (
                  <div className="grid gap-3 border-t border-[var(--color-border)] pt-2">
                    <label className="flex flex-col gap-1 text-sm">
                      Adapter
                      <Select
                        aria-label={`Source ${index + 1} adapter`}
                        value={source.extractor.kind}
                        options={[
                          "search",
                          "greenhouse",
                          "lever",
                          "ashby",
                          "workday",
                          "jsonld",
                          "selector",
                        ].map((value) => ({ value, label: value === "selector" ? "selector / Website crawl" : value }))}
                        onChange={(kind) => {
                          const nextSources = [...draft.sources];
                          nextSources[index] = {
                            ...source,
                            extractor: {
                              ...source.extractor,
                              kind: kind as WorkflowSourceSpec["extractor"]["kind"],
                            },
                          };
                          change({ ...draft, sources: nextSources });
                        }}
                      />
                    </label>
                    {source.extractor.kind === "selector" && (() => {
                      const rawConfig = source.extractor.source_config;
                      const config = rawConfig && typeof rawConfig === "object" && !Array.isArray(rawConfig) ? rawConfig : {};
                      const crawl = config.crawl && typeof config.crawl === "object" && !Array.isArray(config.crawl) ? config.crawl : null;
                      const updateCrawl = (value: typeof config.crawl) => {
                        const nextConfig = { ...config };
                        if (value) nextConfig.crawl = value;
                        else delete nextConfig.crawl;
                        change(current => ({ ...current, sources: current.sources.map(item => item.id === source.id
                          ? { ...item, extractor: { ...item.extractor, mode: "static", source_config: nextConfig } } : item) }));
                      };
                      return <fieldset className="grid gap-3">
                        <legend className="text-sm font-semibold">Website crawl</legend>
                        <label className="flex items-center gap-2 text-sm">
                          <input type="checkbox" aria-label={`Source ${index + 1} website crawl`} checked={!!crawl}
                            onChange={event => updateCrawl(event.target.checked ? { max_pages: 20, max_depth: 2, extraction: "jsonld" } : null)} />
                          Enable bounded website crawl using the source URL above
                        </label>
                        <p className="text-xs text-[var(--color-text-muted)]">Public career/job HTML on the supplied site only, not internet-wide search. Existing feeds and ATS remain available. Pages without supported job fields may yield no postings. Robots and access restrictions still apply.</p>
                        {crawl && <>
                          <label className="grid gap-1 text-sm">Page cap
                            <Input aria-label={`Source ${index + 1} crawl page cap`} type="number" min={1} max={100} value={Number(crawl.max_pages ?? 20)}
                              onChange={event => updateCrawl({ ...crawl, max_pages: Number(event.target.value) })} />
                          </label>
                          <label className="grid gap-1 text-sm">Maximum depth (seed is zero)
                            <Input aria-label={`Source ${index + 1} crawl depth`} type="number" min={0} max={5} value={Number(crawl.max_depth ?? 2)}
                              onChange={event => updateCrawl({ ...crawl, max_depth: Number(event.target.value) })} />
                          </label>
                          <label className="grid gap-1 text-sm">Crawl extraction
                            <Select aria-label={`Source ${index + 1} crawl extraction`} value={String(crawl.extraction ?? "jsonld")}
                              options={[
                                { value: "jsonld", label: "JobPosting JSON-LD" },
                                { value: "public_html", label: "Public HTML (automatic link & card discovery)" },
                                { value: "selectors", label: "Configured selectors" },
                              ]}
                              onChange={extraction => updateCrawl({ ...crawl, extraction })} />
                          </label>
                          <p className="text-xs text-[var(--color-text-muted)]">Static HTML only. Remove next-page pagination before enabling crawl; configured selectors require item, title and link fields.</p>
                        </>}
                      </fieldset>;
                    })()}

                    {source.extractor.kind === "selector" && (
                      <CustomExtractorConfig
                        source={source}
                        sourceIndex={index}
                        initialSource={initialSource}
                        onTrustChanged={onTrustChanged}
                        onSourceChange={(updatedSource) => {
                          const nextSources = [...draft.sources];
                          nextSources[index] = updatedSource;
                          change({ ...draft, sources: nextSources });
                        }}
                      />
                    )}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }
  if (section === "Criteria")
    return (
      <div className="grid gap-4">
        <section aria-label="Posting recency" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <h4 className="font-bold text-sm">Date Posted Window</h4>
          <label className="flex flex-col gap-1 text-sm">
            Recency window
            <Select
              aria-label="Date posted range"
              value={!rules.posting_date ? "any" : rules.posting_date.preset || (rules.posting_date.last_hours != null ? "hours" : "custom")}
              options={[
                { value: "past_12h", label: "Past 12 hours" },
                { value: "today", label: "Today (since local midnight)" },
                { value: "past_24h", label: "Past 24 hours" },
                { value: "past_week", label: "Past week (7 days)" },
                { value: "past_month", label: "Past month (30 days)" },
                { value: "any", label: "Any time" },
                { value: "custom", label: "Custom date range…" },
                { value: "hours", label: "Last N hours (custom)" },
              ]}
              onChange={(val) => {
                const timezone = rules.posting_date?.timezone || schedule.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone;
                const include_unknown = rules.posting_date?.include_unknown ?? false;
                setRules({ posting_date: val === "any" ? null : {
                  preset: val === "custom" || val === "hours" ? null : val,
                  last_hours: val === "hours" ? rules.posting_date?.last_hours ?? 24 : null,
                  from: val === "custom" ? rules.posting_date?.from ?? null : null,
                  through: val === "custom" ? rules.posting_date?.through ?? null : null,
                  timezone, include_unknown,
                } });
              }}
            />
          </label>
          {rules.posting_date && (
            <>
              {!rules.posting_date.preset && rules.posting_date.last_hours == null && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {field(
                    "Posted from",
                    rules.posting_date.from || "",
                    (value) =>
                      setRules({
                        posting_date: {
                          ...rules.posting_date!,
                          preset: null,
                          from: value || null,
                        },
                      }),
                    "date",
                  )}
                  {field(
                    "Posted through",
                    rules.posting_date.through || "",
                    (value) =>
                      setRules({
                        posting_date: {
                          ...rules.posting_date!,
                          preset: null,
                          through: value || null,
                        },
                      }),
                    "date",
                  )}
                </div>
              )}
              {!rules.posting_date.preset && rules.posting_date.last_hours != null && field(
                "Last N hours", rules.posting_date.last_hours,
                value => setRules({ posting_date: { ...rules.posting_date!, last_hours: Number(value) } }), "number",
              )}
              <p className="text-xs text-[var(--color-ink-mute)]">
                Evaluated in {rules.posting_date.timezone}.
              </p>
              {rules.posting_date.preset !== "any" && <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={rules.posting_date.include_unknown}
                  onChange={(e) =>
                    setRules({
                      posting_date: {
                        ...rules.posting_date!,
                        include_unknown: e.target.checked,
                      },
                    })
                  }
                />
                Include jobs with unknown posting dates
              </label>}
            </>
          )}
          {(!rules.posting_date || rules.posting_date.preset === "any") && <p className="text-xs">Any time includes all posting dates, including unknown dates.</p>}
        </section>
        <section aria-label="Location and workplace" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <GeographyMatching rules={rules} onChange={setRules} onDraftChange={() => change(current => ({ ...current }))} />
        </section>
        <section aria-label="Experience, employment and ingestion limits" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-4">
          <h4 className="font-bold text-sm">Experience, Employment & Limits</h4>
          <ExperienceFilter
            experienceMin={rules.experience_min}
            experienceMax={rules.experience_max}
            experienceMatchMode={rules.experience_match_mode}
            includeUnknown={rules.experience_include_unknown}
            newGrad={rules.new_grad ?? "any"}
            onDraftChange={() => change(current => ({ ...current }))}
            onChange={values => setRules({
              experience_min: values.experienceMin, experience_max: values.experienceMax,
              experience_match_mode: values.experienceMatchMode,
              experience_include_unknown: values.includeUnknown, new_grad: values.newGrad,
            })}
          />
          <fieldset className="flex flex-wrap gap-4 border-t border-[var(--color-border)] pt-3 text-sm">
            <legend className="font-bold mb-1">Employment type</legend>
            {(
              [
                ["full_time", "Full-time"],
                ["internship", "Internship"],
                ["contract", "Contract"],
                ["part_time", "Part-time"],
                ["seasonal", "Seasonal"],
                ["unknown", "Unknown employment"],
              ] as const
            ).map(([value, label]) => (
              <label key={value} className="flex min-h-8 items-center gap-2">
                <input
                  type="checkbox"
                  checked={rules.role_types.includes(value)}
                  onChange={(e) => {
                    const updated = e.target.checked
                      ? [...rules.role_types, value]
                      : rules.role_types.filter((v) => v !== value);
                    setRules({ role_types: updated });
                  }}
                />
                {label}
              </label>
            ))}
          </fieldset>
          <p className="text-xs text-[var(--color-ink-mute)]">
            Leave all unchecked to match any role type, including unknown. Select Unknown employment alongside known types to include both.
          </p>
          <div className="border-t border-[var(--color-border)] pt-3">
            <JobLimitInput value={draft.max_jobs_per_run ?? INGESTION_LIMITS.default} onChange={value => change(value == null ? { ...draft } : { ...draft, max_jobs_per_run: value })} />
          </div>
          <CompatibilityFilters draft={draft} change={change} advanced={advanced} />
        </section>
            <label className="grid gap-2 border-t border-[var(--color-border)] pt-3 text-sm">
              Salary matching
              <Select
                aria-label="Salary matching"
                value={compensationMatchMode}
                options={[
                  { value: "range_overlap", label: "Range overlaps threshold" },
                  { value: "advertised_amount", label: "Advertised endpoint meets threshold" },
                ]}
                onChange={(value) =>
                  setRules({
                    compensation: {
                      ...(rules.compensation ?? {
                        min_minor: null,
                        max_minor: null,
                        currency: null,
                        interval: null,
                        include_unknown: false,
                      }),
                      match_mode: value === "advertised_amount" ? "advertised_amount" : "range_overlap",
                    },
                  })
                }
              />
              <span className="text-xs text-[var(--color-ink-mute)]">
                Advertised amount uses only evidenced native pay endpoints; it does not annualize or convert currencies.
              </span>
            </label>
            <SalaryRangeFilter
              salaryMin={
                rules.compensation?.min_minor == null
                  ? null
                  : rules.compensation.min_minor / 100
              }
              salaryMax={
                rules.compensation?.max_minor == null
                  ? null
                  : rules.compensation.max_minor / 100
              }
              currency={rules.compensation?.currency ?? null}
              interval={rules.compensation?.interval ?? null}
              includeUnknown={rules.compensation?.include_unknown ?? false}
              onDraftChange={() => change(current => ({ ...current }))}
              onChange={(values) =>
                setRules({
                  compensation:
                  !values.interval &&
                  !values.currency &&
                  values.salaryMin == null &&
                  values.salaryMax == null
                    ? compensationMatchMode === "range_overlap"
                      ? null
                      : {
                          min_minor: null,
                          max_minor: null,
                          currency: null,
                          interval: null,
                          include_unknown: values.includeUnknown,
                          match_mode: compensationMatchMode,
                        }
                    : {
                        min_minor:
                          values.salaryMin == null
                            ? null
                            : Math.round(values.salaryMin * 100),
                        max_minor:
                          values.salaryMax == null
                            ? null
                            : Math.round(values.salaryMax * 100),
                        currency: values.currency,
                        interval: values.interval,
                        include_unknown: values.includeUnknown,
                        match_mode: compensationMatchMode,
                      },
                })
              }
            />
        {advanced && (
          <>
            {(["sponsorship", "clearance", "work_auth"] as const).map((key) => (
              <fieldset key={key} className="flex flex-wrap gap-3">
                <legend>{key.replaceAll("_", " ")}</legend>
                {(key === "work_auth"
                  ? ["open", "us_person", "citizen_only", "unknown"]
                  : ["yes", "no", "unknown"]
                ).map((value) => (
                  <label key={value}>
                    <input
                      type="checkbox"
                      checked={(rules[key] as string[]).includes(value)}
                      onChange={(e) =>
                        setRules({
                          [key]: e.target.checked
                            ? [...rules[key], value]
                            : rules[key].filter((v) => v !== value),
                        } as Partial<PostingRules>)
                      }
                    />{" "}
                    {value}
                  </label>
                ))}
              </fieldset>
            ))}
            <details><summary className="cursor-pointer min-h-8">Expert geography codes</summary>
            {field("Eligible countries (ISO codes)", rules.countries.join(", "), value => setRules({ countries: words(value).map(code => code.toUpperCase()) }))}
            {field("Eligible regions (ISO codes)", rules.regions.join(", "), value => setRules({ regions: words(value).map(code => code.toUpperCase()) }))}
            </details>
          </>
        )}
      </div>
    );
  if (section === "Schedule")
    return (
      <div className="grid gap-4">
        <section aria-label="Crawl schedule" className="border border-[var(--color-border)] p-3 bg-[var(--color-surface)] grid gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h4 className="font-bold text-sm">Automated Crawl Frequency</h4>
            <div className="flex flex-wrap gap-1.5 text-xs">
              <span className="text-[var(--color-text-muted)] font-semibold uppercase">Presets:</span>
              <button
                type="button"
                aria-pressed={schedule.cadence === "daily" || !schedule.cadence}
                className={`min-h-7 px-2 border text-xs font-semibold uppercase transition-colors ${(schedule.cadence === "daily" || !schedule.cadence) ? "bg-[var(--color-text)] text-[var(--color-surface)] border-[var(--color-border)]" : "border-[var(--color-border)] hover:bg-[var(--color-surface-raised)]"}`}
                onClick={() => setSchedule({ cadence: "daily" })}
              >
                Daily
              </button>
              <button
                type="button"
                aria-pressed={schedule.cadence === "manual"}
                className={`min-h-7 px-2 border text-xs font-semibold uppercase transition-colors ${schedule.cadence === "manual" ? "bg-[var(--color-text)] text-[var(--color-surface)] border-[var(--color-border)]" : "border-[var(--color-border)] hover:bg-[var(--color-surface-raised)]"}`}
                onClick={() => setSchedule({ cadence: "manual" })}
              >
                Manual
              </button>
            </div>
          </div>
          <label className="flex flex-col gap-1 text-sm">
            Schedule
            <Select
              aria-label="Schedule"
              value={schedule.cadence || "daily"}
              options={["daily", "weekly", "manual", "hourly", "every_6h", "every_12h", "cron"].map(value => ({ value, label: value }))}
              onChange={(cadence) =>
                setSchedule({
                  cadence: cadence as NonNullable<
                    WorkflowSpec["schedule"]
                  >["cadence"],
                })
              }
            />
          </label>
        {["daily", "weekly"].includes(schedule.cadence || "") && (
          <>
            <div role="group" aria-label="Local run time" className="grid gap-2">
              <span className="text-sm">Local run time</span>
              <ClockPicker
                value={clockValue}
                onChange={(value) => setSchedule({ time: `${pad2(to24Hour(value.hour12, value.ampm))}:${pad2(value.minute)}` })}
                is24h={is24h}
                hideCadenceOptions
              />
            </div>
            {advanced && (
              <fieldset className="flex flex-wrap gap-3">
                <legend>Weekdays (none means every day)</legend>
                {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(
                  (day, i) => (
                    <label key={day}>
                      <input
                        type="checkbox"
                        checked={(
                          schedule.weekdays ||
                          (schedule.cadence === "weekly" ? [1] : [])
                        ).includes(i + 1)}
                        onChange={(e) =>
                          setSchedule({
                            weekdays: e.target.checked
                              ? [...(schedule.weekdays || []), i + 1]
                              : (schedule.weekdays || []).filter(
                                  (d) => d !== i + 1,
                                ),
                          })
                        }
                      />{" "}
                      {day}
                    </label>
                  ),
                )}
              </fieldset>
            )}
          </>
        )}
        {schedule.cadence === "cron" &&
          field("Cron expression", schedule.cron || "", (cron) =>
            setSchedule({ cron }),
          )}
        {field(
          "IANA timezone",
          schedule.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone,
          (timezone) => {
            const next = { ...draft, schedule: { ...schedule, timezone } };
            if (rules.posting_date)
              next.filters = {
                ...filters,
                matching: {
                  ...rules,
                  posting_date: { ...rules.posting_date, timezone },
                },
              };
            change(next);
          },
        )}
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            aria-label="Run this workflow when the app starts"
            checked={schedule.run_on_startup ?? false}
            onChange={(event) => setSchedule({ run_on_startup: event.target.checked })}
          />
          Run this workflow when the app starts
        </label>
        <label>
          <input
            type="checkbox"
            checked={draft.metadata.enabled ?? true}
            onChange={(e) =>
              change({
                ...draft,
                metadata: { ...draft.metadata, enabled: e.target.checked },
              })
            }
          />{" "}
          Workflow enabled
        </label>
        <p className="text-sm">
          Automatic runs also respect the global scheduling pause. Manual runs
          remain available.
        </p>
        </section>
      </div>
    );
  return (
    <div className="grid gap-3">
      {field(
        "Retain untracked discoveries for days (0 keeps forever)",
        draft.retention?.days ?? 30,
        (value) =>
          change({
            ...draft,
            retention: {
              ...draft.retention,
              days: Number(value),
              policy: "untracked_only",
            },
          }),
        "number",
      )}
      <p className="text-sm">
        Tracked jobs, favorites, notes, and removed or blocked records are
        always protected.
      </p>
    </div>
  );
}
