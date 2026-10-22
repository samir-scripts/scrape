import type { PostingRules } from "../../../bindings/PostingRules";
import type { KeywordRules } from "../../../bindings/KeywordRules";
import type { FilterField } from "../../../bindings/FilterField";
import type { WorkflowDraftChange } from "../../../hooks/useWorkflowDraft";
import { KeywordInput } from "../../ui/KeywordInput";
const defaultKeywords = (): KeywordRules => ({ any: [], all: [], exclude: [], fields: ["title"] });
const fields = ["title", "description", "company", "location"] as const;
export function KeywordMatching({ rules: postingRules, change }: { rules: PostingRules; change: WorkflowDraftChange }) {
  const rules = postingRules.keywords || defaultKeywords();
  const patch = (update: (current: KeywordRules) => KeywordRules) => change(current => ({
    ...current, filters: { ...current.filters, matching: { ...postingRules, ...current.filters?.matching,
      keywords: update(current.filters?.matching?.keywords || defaultKeywords()),
    } },
  }));
  const groups = ["any", "all", "exclude"] as const;
  const terms = (group: typeof groups[number], label: string) => {
    const suggestions = [...(postingRules.target_roles || []), ...groups.filter(other => other !== group).flatMap(other => rules[other])];
    const seen = new Set<string>();
    const unique = suggestions.filter(term => {
      const key = term.toLocaleLowerCase();
      if (rules[group].some(existing => existing.toLocaleLowerCase() === key) || seen.has(key)) return false;
      seen.add(key); return true;
    });
    return <KeywordInput label={label} value={rules[group]} suggestions={unique}
      onChange={value => patch(current => ({ ...current, [group]: value }))}
      onDraftChange={() => change(current => ({ ...current }))} />;
  };
  const searchFields = rules.fields.length ? rules.fields : fields;
  return <div className="grid gap-3">
    <p className="text-sm text-[var(--color-ink-mute)]">Enter a phrase, then press Enter or Add. Commas and newlines also add terms; spaces stay within a phrase.</p>
    {terms("any", "Any keywords")}
    {terms("all", "All keywords")}
    {terms("exclude", "Exclude keywords")}
    <p className="text-sm text-[var(--color-ink-mute)]">When Any is nonempty, at least one Any term is required. Every All term is required, and every Exclude term must be absent. Both groups apply together. Machine Learning stays one phrase. Substrings match: Engineer also matches Engineering.</p>
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
