import { useEffect, useId, useRef, useState } from "react";
import { Button } from "./Button";
import { Input } from "./Input";

// An editor action flushes every mounted input synchronously before reading its draft.
export const COMMIT_KEYWORDS = "scrape:commit-keywords";
export function keywordTerms(existing: readonly string[], text: string): string[] {
  const seen = new Set<string>();
  return [...existing, ...text.split(/[,\r\n]+/).map(term => term.trim())].filter(term => {
    const key = term.toLowerCase();
    if (!term || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export function KeywordInput({ label, value, onChange, onDraftChange }: {
  label: string; value: string[]; onChange: (terms: string[]) => void; onDraftChange: () => void;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const removers = useRef<(HTMLButtonElement | null)[]>([]);
  const [text, setText] = useState("");
  const pending = useRef("");
  const composing = useRef(false);
  const current = useRef({ value, onChange });
  current.current = { value, onChange };
  const updateText = (next: string) => { pending.current = next; setText(next); };
  const add = (next: string) => {
    if (!next.trim()) return;
    const terms = keywordTerms(current.current.value, next);
    if (JSON.stringify(terms) !== JSON.stringify(current.current.value)) {
      current.current.value = terms;
      current.current.onChange(terms);
    }
  };
  const commit = () => { add(pending.current); updateText(""); };
  const commitRef = useRef(commit); commitRef.current = commit;
  useEffect(() => {
    const field = input.current;
    const flush = () => commitRef.current();
    field?.addEventListener(COMMIT_KEYWORDS, flush);
    return () => field?.removeEventListener(COMMIT_KEYWORDS, flush);
  }, []);
  const edit = (next: string) => {
    onDraftChange();
    if (!composing.current && /[,\r\n]/.test(next)) {
      const parts = next.split(/[,\r\n]/);
      updateText(parts.pop() || "");
      add(parts.join(","));
    } else updateText(next);
  };
  const remove = (index: number) => {
    const next = current.current.value.filter((_, position) => position !== index);
    current.current.value = next;
    current.current.onChange(next);
    requestAnimationFrame(() => (removers.current[index - 1] || input.current)?.focus());
  };
  return <div className="grid min-w-0 gap-2">
    <label htmlFor={id} className="text-sm">{label}</label>
    <span className="sr-only" role="status">{value.length} terms in {label}</span>
    {!!value.length && <ul aria-label={`${label} terms`} className="flex max-h-40 flex-wrap gap-2 overflow-y-auto">
      {value.map((term, index) => <li key={`${index}:${term}`} className="inline-flex max-w-full items-center gap-1 border bg-[var(--color-paper-2)] pl-2 text-sm">
        <span className="break-words min-w-0 normal-case">{term}</span>
        <button type="button" ref={node => { removers.current[index] = node; }}
          aria-label={`Remove ${term} from ${label}`} className="min-h-8 min-w-8 shrink-0 cursor-pointer"
          onClick={() => remove(index)} onKeyDown={event => {
            if (event.key === "Backspace" || event.key === "Delete") { event.preventDefault(); remove(index); }
          }}>
          <svg aria-hidden="true" viewBox="0 0 16 16" width="14" height="14" className="mx-auto" fill="none" stroke="currentColor"><path d="m4 4 8 8M12 4l-8 8" /></svg>
        </button>
      </li>)}
    </ul>}
    <div className="flex min-w-0 items-start gap-2">
      <Input ref={input} id={id} data-keyword-input aria-label={label} value={text} placeholder="Type a phrase"
        onChange={event => edit(event.target.value)} onBlur={commit}
        onCompositionStart={() => { composing.current = true; }}
        onCompositionEnd={event => { composing.current = false; edit(event.currentTarget.value); }}
        onPaste={event => {
          const pasted = event.clipboardData.getData("text");
          if (!/[,\r\n]/.test(pasted) || composing.current) return;
          event.preventDefault();
          const field = event.currentTarget;
          const combined = pending.current.slice(0, field.selectionStart ?? pending.current.length) + pasted + pending.current.slice(field.selectionEnd ?? pending.current.length);
          onDraftChange(); add(combined); updateText("");
        }}
        onKeyDown={event => {
          if (composing.current || event.nativeEvent.isComposing || event.keyCode === 229) return;
          if (event.key === "Enter") { event.preventDefault(); event.stopPropagation(); commit(); }
          if (event.key === "Backspace" && !pending.current && current.current.value.length) {
            event.preventDefault(); removers.current[current.current.value.length - 1]?.focus();
          }
        }} />
      <Button type="button" aria-label={`Add keyword to ${label}`} onMouseDown={event => event.preventDefault()} onClick={() => { commit(); input.current?.focus(); }}>Add</Button>
    </div>
  </div>;
}
