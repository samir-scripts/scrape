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
import { api } from "../../../lib/api";
import { canonicalizeAtsUrl, normalizedEndpoint } from "../../../lib/yamlWorkflow";

export const emptyRules = (): PostingRules => ({
  keywords: null,
  posting_date: null,
  compensation: null,
  role_types: [],
  workplaces: [],
  experience_min: null,
  experience_max: null,
  experience_include_unknown: false,
  new_grad: null,
  sponsorship: [],
  work_auth: [],
  clearance: [],
  countries: [],
  regions: [],
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
}: {
  onSourceBlur?: () => void;
  draft: WorkflowSpec;
  change: WorkflowDraftChange;
  advanced: boolean;
  section: "Source" | "Matching" | "Schedule" | "Retention";
  initialSource?: Source | null;
  onTrustChanged?: () => void;
}) {
  const filters = draft.filters || {};
  const rules = { ...emptyRules(), ...filters.matching };
  const setRules = (patch: Partial<PostingRules>) =>
    change({
      ...draft,
      filters: { ...filters, matching: { ...rules, ...patch } },
    });
  const schedule = draft.schedule || {};
  const setSchedule = (patch: Partial<NonNullable<WorkflowSpec["schedule"]>>) =>
    change({ ...draft, schedule: { ...schedule, ...patch } });
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
  if (section === "Source") {
    const endpointMap = new Map<string, number>();
    (draft.sources || []).forEach((s) => {
      if (s.extractor?.url?.trim()) {
        const ep = normalizedEndpoint(s.extractor.kind, s.extractor.url);
        endpointMap.set(ep, (endpointMap.get(ep) || 0) + 1);
      }
    });
    const hasDuplicates = Array.from(endpointMap.values()).some((c) => c > 1);
    const allDisabled = (draft.sources?.length ?? 0) > 0 && !draft.sources.some((s) => s.enabled);

    return (
      <div className="grid gap-4">
        {field("Workflow name", draft.metadata.name, (name) =>
          change({ ...draft, metadata: { ...draft.metadata, name } }),
        )}
        {field(
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

              <div className="grid gap-3">
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

                <div className="flex flex-col gap-1 text-sm">
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
                      aria-label={`Detect source ${index + 1}`}
                      onClick={async () => {
                        if (!source.extractor.url.trim()) return;
                        try {
                          const res = await api.detectSource(source.extractor.url);
                          const url = canonicalizeAtsUrl(res.kind, res.identifier);
                          const isDefault = !source.label || source.label.startsWith("Source ") || source.label.startsWith("Career Board");
                          const nextSources = [...draft.sources];
                          nextSources[index] = {
                            ...source,
                            label: isDefault && res.name ? res.name : source.label,
                            employer_hint: source.employer_hint || res.name || undefined,
                            extractor: {
                              ...source.extractor,
                              kind: res.kind as WorkflowSourceSpec["extractor"]["kind"],
                              url,
                            },
                          };
                          change({
                            ...draft,
                            metadata: {
                              ...draft.metadata,
                              name: draft.metadata.name || res.name,
                              company: draft.metadata.company || res.name,
                            },
                            sources: nextSources,
                          });
                        } catch {
                          // ignore or let user configure manually
                        }
                      }}
                    >
                      Detect
                    </Button>
                  </div>
                </div>

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

                {advanced && (
                  <div className="grid gap-3 border-t border-[var(--color-border)] pt-2">
                    <label className="flex flex-col gap-1 text-sm">
                      Adapter
                      <Select
                        aria-label={`Source ${index + 1} adapter`}
                        value={source.extractor.kind}
                        options={[
                          "greenhouse",
                          "lever",
                          "ashby",
                          "jsonld",
                          "selector",
                        ].map((value) => ({ value, label: value }))}
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
  if (section === "Matching")
    return (
      <div className="grid gap-3">
        <JobLimitInput value={draft.max_jobs_per_run ?? INGESTION_LIMITS.default} onChange={value => change(value == null ? { ...draft } : { ...draft, max_jobs_per_run: value })} />
        <KeywordMatching rules={rules} change={change} />
        <GeographyMatching rules={rules} onChange={setRules} onDraftChange={() => change({ ...draft })} />
        {advanced && field(
          "Location matches (additional text restriction)",
          filters.locations?.join(", ") || "",
          (value) =>
            change({
              ...draft,
              filters: { ...filters, locations: words(value) },
            }),
        )}
        {advanced && (
          <>
            <fieldset className="flex flex-wrap gap-3">
              <legend>Roles</legend>
              {(
                [
                  "full_time",
                  "part_time",
                  "contract",
                  "internship",
                  "seasonal",
                ] as const
              ).map((value) => (
                <label key={value}>
                  <input
                    type="checkbox"
                    checked={rules.role_types.includes(value)}
                    onChange={(e) =>
                      setRules({
                        role_types: e.target.checked
                          ? [...rules.role_types, value]
                          : rules.role_types.filter((v) => v !== value),
                      })
                    }
                  />{" "}
                  {value.replaceAll("_", " ")}
                </label>
              ))}
            </fieldset>
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
              onDraftChange={() => change(draft)}
              onChange={(values) =>
                setRules({
                  compensation:
                    !values.interval &&
                    !values.currency &&
                    values.salaryMin == null &&
                    values.salaryMax == null
                      ? null
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
                        },
                })
              }
            />
            {field(
              "Experience minimum (years)",
              rules.experience_min ?? "",
              (value) =>
                setRules({ experience_min: value ? Number(value) : null }),
              "number",
            )}
            {field(
              "Experience maximum (years)",
              rules.experience_max ?? "",
              (value) =>
                setRules({ experience_max: value ? Number(value) : null }),
              "number",
            )}
            <label>
              <input
                type="checkbox"
                checked={rules.experience_include_unknown}
                onChange={(e) =>
                  setRules({ experience_include_unknown: e.target.checked })
                }
              />{" "}
              Include unknown experience
            </label>
            <label>
              New graduate roles
              <Select
                aria-label="New graduate roles"
                value={rules.new_grad || "any"}
                options={["any", "only", "exclude"].map((value) => ({
                  value,
                  label: value,
                }))}
                onChange={(new_grad) => setRules({ new_grad })}
              />
            </label>
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
            <label>
              Posting date restriction
              <Select
                aria-label="Posting date restriction"
                value={
                  !rules.posting_date
                    ? "none"
                    : rules.posting_date.last_hours != null
                      ? "age"
                      : "range"
                }
                options={[
                  { value: "none", label: "No date restriction" },
                  { value: "age", label: "Last N hours" },
                  { value: "range", label: "Date range" },
                ]}
                onChange={(range) =>
                  setRules({
                    posting_date:
                      range === "none"
                        ? null
                        : {
                            last_hours: range === "age" ? 24 : null,
                            from: null,
                            through: null,
                            timezone:
                              schedule.timezone ||
                              Intl.DateTimeFormat().resolvedOptions().timeZone,
                            include_unknown: false,
                          },
                  })
                }
              />
            </label>
            {rules.posting_date && (
              <>
                {rules.posting_date.last_hours != null ? (
                  field(
                    "Posted in last hours",
                    rules.posting_date.last_hours,
                    (value) =>
                      setRules({
                        posting_date: {
                          ...rules.posting_date!,
                          last_hours: Number(value),
                        },
                      }),
                    "number",
                  )
                ) : (
                  <>
                    {field(
                      "Posted from",
                      rules.posting_date.from || "",
                      (value) =>
                        setRules({
                          posting_date: {
                            ...rules.posting_date!,
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
                            through: value || null,
                          },
                        }),
                      "date",
                    )}
                  </>
                )}
                <p>Date boundaries use {rules.posting_date.timezone}.</p>
                <label>
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
                  />{" "}
                  Include unknown posting dates
                </label>
              </>
            )}
            <details><summary className="cursor-pointer min-h-8">Expert geography codes</summary>
            {field("Eligible countries (ISO codes)", rules.countries.join(", "), value => setRules({ countries: words(value).map(code => code.toUpperCase()) }))}
            {field("Eligible regions (ISO codes)", rules.regions.join(", "), value => setRules({ regions: words(value).map(code => code.toUpperCase()) }))}
            </details>
            {field("Expert query", filters.query || "", (query) =>
              change({
                ...draft,
                filters: { ...filters, query: query || undefined },
              }),
            )}
          </>
        )}
      </div>
    );
  if (section === "Schedule")
    return (
      <div className="grid gap-3">
        <label>
          Schedule
          <Select
            aria-label="Schedule"
            value={schedule.cadence || "daily"}
            options={(advanced
              ? [
                  "daily",
                  "weekly",
                  "manual",
                  "hourly",
                  "every_6h",
                  "every_12h",
                  "cron",
                ]
              : [
                  "daily",
                  "manual",
                  ...(schedule.cadence &&
                  !["daily", "manual"].includes(schedule.cadence)
                    ? [schedule.cadence]
                    : []),
                ]
            ).map((value) => ({
              value,
              label: value,
            }))}
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
            {field(
              "Local run time",
              schedule.time || "09:00",
              (time) => setSchedule({ time }),
              "time",
            )}
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
