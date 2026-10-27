import { useEffect, useState } from "react";
import type { FilterCompatibilityRestriction } from "../../../bindings/FilterCompatibilityRestriction";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowDraftChange } from "../../../hooks/useWorkflowDraft";
import { api } from "../../../lib/api";
import { errorMessage } from "../../../lib/errors";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { KeywordInput } from "../../ui/KeywordInput";

export function CompatibilityFilters({ draft, change, advanced }: { draft: WorkflowSpec; change: WorkflowDraftChange; advanced: boolean }) {
  const filters = draft.filters || {};
  const [descriptions, setDescriptions] = useState<FilterCompatibilityRestriction[]>([]);
  const [error, setError] = useState("");
  const serialized = JSON.stringify(filters);
  const active = !!(filters.include_keywords?.length || filters.exclude_keywords?.length || filters.locations?.length || filters.role_types?.length || filters.query || filters.policy || filters.compensation);
  useEffect(() => {
    let cancelled = false;
    setDescriptions([]); setError("");
    if (active) {
      api.describeWorkflowFilters(JSON.parse(serialized)).then(result => {
        if (!cancelled) setDescriptions(result);
      }).catch(reason => { if (!cancelled) setError(errorMessage(reason)); });
    }
    return () => { cancelled = true; };
  }, [serialized, active]);
  if (!active && !advanced) return null;
  const update = (patch: Partial<NonNullable<WorkflowSpec["filters"]>>) => change(current => ({ ...current, filters: { ...current.filters, ...patch } }));
  const explain = (field: string) => descriptions.find(entry => entry.field === field)?.explanation;
  return <section aria-label="Compatibility restrictions" className="grid gap-3 border p-3">
    <h3 className="font-bold">Additional compatibility restrictions</h3>
    <p className="text-sm">These saved restrictions apply together with the structured rules below. They are preserved without conversion. Editing or removing one changes matching; review the next preview before saving.</p>
    {error && <p role="status" className="text-xs text-[var(--color-ink-mute)]">{error}</p>}
    {([
      ["include_keywords", "Legacy include keywords"],
      ["exclude_keywords", "Legacy exclude keywords"],
      ["locations", "Legacy location terms"],
    ] as const).map(([key, label]) => !!filters[key]?.length && <div key={key} className="grid gap-1">
      <KeywordInput actionId="workflow.legacy_terms" label={label} value={filters[key] || []} onDraftChange={() => change(current => ({ ...current }))} onChange={terms => update({ [key]: terms })} />
      <p className="text-xs">{explain(key)}</p>
    </div>)}
    {!!filters.role_types?.length && <div className="grid gap-1">
      <p className="text-sm">Legacy role types: {filters.role_types.join(", ")}</p>
      <p className="text-xs">{explain("role_types")}</p>
      <Button data-control-action="workflow.remove_legacy_role_types" type="button" size="sm" onClick={() => update({ role_types: [] })}>Remove legacy role-type restriction</Button>
      <p className="text-xs">Edit individual legacy role types in YAML.</p>
    </div>}
    {(!!filters.query || advanced) && <label className="grid gap-1 text-sm">Expert query
      <Input data-control-action="workflow.expert_query" aria-label="Expert query" value={filters.query || ""} onChange={event => update({ query: event.target.value || null })} />
      <span className="text-xs">{explain("query")}</span>
    </label>}
    {(!!filters.policy || advanced) && <label className="grid gap-1 text-sm">Expert policy (JSON)
      <Input data-control-action="workflow.expert_policy" aria-label="Expert policy (JSON)" value={typeof filters.policy === "string" ? filters.policy : filters.policy ? JSON.stringify(filters.policy) : ""} onChange={event => {
        const text = event.target.value;
        if (!text) { update({ policy: null }); return; }
        try { update({ policy: JSON.parse(text) }); }
        catch { update({ policy: text }); }
      }} />
      <span className="text-xs">{explain("policy")}</span>
    </label>}
    {!!filters.compensation && <div className="grid gap-1 text-sm">
      <p>Legacy compensation: <code className="break-all">{JSON.stringify(filters.compensation)}</code></p>
      <p className="text-xs">{explain("compensation")} Edit this restriction in YAML.</p>
      <Button data-control-action="workflow.remove_legacy_compensation" type="button" size="sm" onClick={() => update({ compensation: null })}>Remove legacy compensation restriction</Button>
    </div>}
  </section>;
}
