import { useState } from "react";
import type { PostingRules } from "../../../bindings/PostingRules";
import type { KeywordRules } from "../../../bindings/KeywordRules";
import type { FilterField } from "../../../bindings/FilterField";
import type { WorkflowDraftChange } from "../../../hooks/useWorkflowDraft";
import { KeywordInput } from "../../ui/KeywordInput";
import { Select } from "../../ui/Select";
const defaultKeywords = (): KeywordRules => ({ any: [], all: [], exclude: [], fields: ["title"] });
const fields = ["title", "description", "company", "location"] as const;
export function KeywordMatching({ rules: postingRules, change }: { rules: PostingRules; change: WorkflowDraftChange }) {
  const rules = postingRules.keywords || defaultKeywords();
  const [emptyMode, setEmptyMode] = useState<"any" | "all">(() => rules.all.length && !rules.any.length ? "all" : "any");
  // Retain both imported groups while editing, including temporarily empty groups.
  const [mixed] = useState(() => rules.any.length > 0 && rules.all.length > 0);
  const mode = rules.all.length ? "all" : rules.any.length ? "any" : emptyMode;
  const patch = (update: (current: KeywordRules) => KeywordRules) => change(current => ({
    ...current, filters: { ...current.filters, matching: { ...postingRules, ...current.filters?.matching,
      keywords: update(current.filters?.matching?.keywords || defaultKeywords()),
    } },
  }));
  const terms = (group: "any" | "all" | "exclude", label: string) => <KeywordInput label={label} value={rules[group]}
    onChange={value => patch(current => ({ ...current, [group]: value }))}
    onDraftChange={() => change(current => ({ ...current }))} />;
  const searchFields = rules.fields.length ? rules.fields : fields;
  return <div className="grid gap-3">
    <p className="text-sm text-[var(--color-ink-mute)]">Enter a phrase, then press Enter or Add. Commas and newlines also add terms; spaces stay within a phrase.</p>
    {mixed ? <>
      <p>Imported rules require an Any match and every All term, with no excluded term.</p>
      {terms("any", "Any keywords")}
      {terms("all", "All keywords")}
    </> : <>
      <label className="grid gap-1 text-sm">Match
        <Select aria-label="Match keywords" value={mode} options={[{ value: "any", label: "Any keyword" }, { value: "all", label: "All keywords" }]}
          onChange={value => {
            const next = value as "any" | "all";
            setEmptyMode(next);
            patch(current => ({ ...current, [next]: [...current.any, ...current.all], [next === "any" ? "all" : "any"]: [] }));
          }} />
      </label>
      {terms(mode, "Keywords")}
    </>}
    {terms("exclude", "Exclude keywords")}
    <p className="text-sm text-[var(--color-ink-mute)]">Any: Engineer, Developer matches either. All: Software, Engineer requires both. Machine Learning stays one phrase. Substrings match: Engineer also matches Engineering.</p>
    <fieldset className="flex flex-wrap gap-3">
      <legend>Search in</legend>
      {fields.map(field => <label key={field} className="flex min-h-8 items-center gap-2 capitalize">
        <input type="checkbox" checked={searchFields.includes(field)}
          disabled={searchFields.length === 1 && searchFields.includes(field)}
          onChange={event => {
            const checked = event.target.checked;
            patch(current => {
              const effective: readonly FilterField[] = current.fields.length ? current.fields : fields;
              return { ...current, fields: checked ? [...effective, field] : effective.filter(value => value !== field) };
            });
          }} />{field}
      </label>)}
    </fieldset>
  </div>;
}
