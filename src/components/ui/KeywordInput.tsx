import { useEffect, useId, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
// Suggestions use the same listbox placement, while plain keyword fields stay textboxes.
import { Button } from "./Button";
import { Input } from "./Input";
import { PopupPosition, positionOptions } from "./Select";
import { recordControlAction } from "../../lib/controlActivity";
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

export function KeywordInput({ label, value, onChange, onDraftChange, suggestions, actionId }: {
  label: string; value: string[]; onChange: (terms: string[]) => void; onDraftChange: () => void; suggestions?: readonly string[]; actionId?: string;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const removers = useRef<(HTMLButtonElement | null)[]>([]);
  const [text, setText] = useState("");
  const pending = useRef("");
  const composing = useRef(false);
  const current = useRef({ value, onChange });
  current.current = { value, onChange };
  const root = useRef<HTMLDivElement>(null);
  const [popupTarget, setPopupTarget] = useState<HTMLElement | null>(null);
  const [active, setActive] = useState(-1);
  const [open, setOpen] = useState(false);
  const matchingSuggestions = useMemo(() => {
    const query = text.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
    return [...new Set(suggestions ?? [])].filter(term => !value.some(item => item.toLocaleLowerCase() === term.toLocaleLowerCase()))
      .filter(term => query.every(token => term.toLocaleLowerCase().includes(token)))
      .sort((a, b) => a.localeCompare(b)).slice(0, 20);
  }, [suggestions, text, value]);
  const complete = (term: string) => {
    onDraftChange();
    add(term);
    updateText("");
    setOpen(false);
    setActive(-1);
    requestAnimationFrame(() => input.current?.focus());
  };
  const updateText = (next: string) => { pending.current = next; setText(next); };
  const add = (next: string, userAction = true) => {
    if (!next.trim()) return;
    const terms = keywordTerms(current.current.value, next);
    if (JSON.stringify(terms) !== JSON.stringify(current.current.value)) {
      if (userAction && actionId) recordControlAction(actionId, "change");
      current.current.value = terms;
      current.current.onChange(terms);
    }
  };
  const commit = (userAction = true) => { add(pending.current, userAction); updateText(""); };
  const commitRef = useRef(commit); commitRef.current = commit;
  useEffect(() => {
    const field = input.current;
    const flush = () => commitRef.current(false);
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
    if (actionId) recordControlAction(actionId, "change");
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
        <button type="button" data-control-action={actionId} data-control-semantic ref={node => { removers.current[index] = node; }}
          aria-label={`Remove ${term} from ${label}`} className="min-h-8 min-w-8 shrink-0 cursor-pointer"
          onClick={() => remove(index)} onKeyDown={event => {
            if (event.key === "Backspace" || event.key === "Delete") { event.preventDefault(); remove(index); }
          }}>
          <svg aria-hidden="true" viewBox="0 0 16 16" width="14" height="14" className="mx-auto" fill="none" stroke="currentColor"><path d="m4 4 8 8M12 4l-8 8" /></svg>
        </button>
      </li>)}
    </ul>}
    <PopupPosition root={root} open={open && !!suggestions?.length} onTarget={setPopupTarget}>
      <div ref={root} className="flex min-w-0 items-start gap-2">
        <Input ref={input} id={id} data-keyword-input data-control-action={actionId} data-control-semantic aria-label={label} value={text} placeholder="Type a phrase"
          role={suggestions ? "combobox" : undefined} aria-autocomplete={suggestions ? "list" : undefined}
          aria-expanded={suggestions ? open && !!matchingSuggestions.length : undefined}
          aria-controls={suggestions ? `${id}-suggestions` : undefined}
          aria-activedescendant={suggestions && active >= 0 ? `${id}-suggestion-${active}` : undefined}
          onChange={event => { edit(event.target.value); setOpen(!!suggestions?.length); setActive(-1); }}
          onBlur={() => { commit(); setOpen(false); }}
          onCompositionStart={() => { composing.current = true; }}
          onCompositionEnd={event => { composing.current = false; edit(event.currentTarget.value); setOpen(!!suggestions?.length); }}
          onPaste={event => {
            const pasted = event.clipboardData.getData("text");
            if (!/[,\r\n]/.test(pasted) || composing.current) return;
            event.preventDefault();
            const field = event.currentTarget;
            const combined = pending.current.slice(0, field.selectionStart ?? pending.current.length) + pasted + pending.current.slice(field.selectionEnd ?? pending.current.length);
            onDraftChange(); add(combined); updateText(""); setOpen(false);
          }}
          onKeyDown={event => {
            if (composing.current || event.nativeEvent.isComposing || event.keyCode === 229) return;
            if (suggestions && open && matchingSuggestions.length && event.key === "ArrowDown") { event.preventDefault(); setActive(index => (index + 1) % matchingSuggestions.length); return; }
            if (suggestions && open && matchingSuggestions.length && event.key === "ArrowUp") { event.preventDefault(); setActive(index => index <= 0 ? matchingSuggestions.length - 1 : index - 1); return; }
            if (suggestions && open && event.key === "Escape") { event.preventDefault(); event.stopPropagation(); setOpen(false); setActive(-1); return; }
            if (event.key === "Enter") {
              event.preventDefault(); event.stopPropagation();
              if (suggestions && open && active >= 0 && matchingSuggestions[active]) complete(matchingSuggestions[active]);
              else commit();
            }
            if (event.key === "Backspace" && !pending.current && current.current.value.length) {
              event.preventDefault(); removers.current[current.current.value.length - 1]?.focus();
            }
          }} />
        {popupTarget && suggestions && open && matchingSuggestions.length > 0 && createPortal(
          <ul id={`${id}-suggestions`} role="listbox" data-control-action={actionId} data-control-semantic className="fixed z-[100] max-h-60 overflow-auto border border-[var(--color-ink)] bg-[var(--color-paper)] p-1 text-[var(--color-ink)]" style={positionOptions(root.current)}>
            {matchingSuggestions.map((term, index) => <li id={`${id}-suggestion-${index}`} key={term} role="option" aria-selected={active === index}
              className={`cursor-pointer px-2 py-2 text-sm normal-case ${active === index ? "bg-[var(--color-paper-2)]" : ""}`}
              onMouseDown={event => event.preventDefault()} onMouseEnter={() => setActive(index)} onClick={() => complete(term)}>{term}</li>)}
          </ul>, popupTarget)}
        <Button type="button" data-control-action={actionId} data-control-semantic aria-label={`Add keyword to ${label}`} onMouseDown={event => event.preventDefault()} onClick={() => { commit(); setOpen(false); input.current?.focus(); }}>Add</Button>
      </div>
    </PopupPosition>
  </div>;
}
