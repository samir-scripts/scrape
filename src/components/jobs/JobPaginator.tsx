import { useEffect, useId, useRef, useState } from "react";
import { pagination } from "../../lib/pagination";
import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";

export function JobPaginator({ page, pageSize, total, busy = false, onPageChange }: {
  page: number; pageSize: number; total: number | undefined; busy?: boolean; onPageChange: (page: number) => void;
}) {
  const model = pagination(total, page, pageSize);
  const [draft, setDraft] = useState(String(model.page));
  const [error, setError] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const errorId = useId(), helpId = useId();
  useEffect(() => { setDraft(String(model.page)); setError(""); }, [model.page, total]);
  const disabled = !model.pageCount;
  const submit = () => {
    const value = Number(draft.trim());
    if (!/^\d+$/.test(draft.trim()) || !Number.isSafeInteger(value) || value < 1 || value > (model.pageCount ?? 0)) {
      setError(`Enter a whole page number from 1 to ${model.pageCount}.`);
      input.current?.focus(); return;
    }
    setError(""); setDraft(String(value));
    if (value !== model.page) onPageChange(value);
  };
  return <nav aria-label="Job pagination" className="shrink-0 border-t border-[var(--color-border)] bg-[var(--color-paper)] px-3 py-2 text-sm">
    <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
      <span role="status" aria-live="polite" aria-atomic="true">{busy ? `Loading page ${page}…` : total === undefined ? "Results unavailable" : `${model.start}–${model.end} of ${total}`}</span>
      <div className="flex flex-wrap items-center gap-1">
        <IconButton label="First page" disabled={disabled || model.page === 1} onClick={() => onPageChange(1)}><Icon name="first" /></IconButton>
        <IconButton label="Previous page" disabled={disabled || model.page === 1} onClick={() => onPageChange(model.page - 1)}><Icon name="left" /></IconButton>
        <form aria-label="Jump to page" onSubmit={event => { event.preventDefault(); submit(); }} className="flex items-center gap-2 px-1">
          <label className="normal-case">Page <input ref={input} aria-label="Page number" inputMode="numeric" type="text" disabled={disabled}
            value={total === 0 ? "" : draft} aria-invalid={Boolean(error)} aria-describedby={`${helpId}${error ? ` ${errorId}` : ""}`}
            onChange={event => { setDraft(event.target.value); setError(""); }}
            onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); setDraft(String(model.page)); setError(""); } }}
            className="w-14 min-h-8 border border-[var(--color-border)] bg-[var(--color-paper)] p-1 text-center" /></label>
          <span>of {model.pageCount ?? "—"}</span>
          <IconButton label="Go to page" type="submit" disabled={disabled}><Icon name="right" /></IconButton>
        </form>
        <IconButton label="Next page" disabled={disabled || model.page === model.pageCount} onClick={() => onPageChange(model.page + 1)}><Icon name="right" /></IconButton>
        <IconButton label="Last page" disabled={disabled || model.page === model.pageCount} onClick={() => onPageChange(model.pageCount!)}><Icon name="last" /></IconButton>
      </div>
    </div>
    <span id={helpId} className="sr-only">Enter a page number and press Enter, or use Go to page.</span>
    {error && <p id={errorId} role="alert" className="mt-1 text-[var(--color-danger)]">{error}</p>}
  </nav>;
}
