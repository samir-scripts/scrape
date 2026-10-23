import { createPortal } from "react-dom";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";
import { PopupPosition, positionOptions } from "./Select";

export interface MultiSelectOption {
  value: string;
  label: string;
  searchText?: string;
  description?: string;
}

const normalize = (value: string) => value.trim().toLocaleLowerCase();

export function MultiSelectCombobox({ label, value, options, onChange, placeholder, allowCustom = false, disabled, describedBy, loadOptions }: {
  label: string; value: string[]; options: readonly MultiSelectOption[];
  onChange: (values: string[]) => void; placeholder?: string; allowCustom?: boolean;
  disabled?: boolean; describedBy?: string;
  loadOptions?: (query: string) => Promise<readonly MultiSelectOption[]>;
}): React.JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const removers = useRef<(HTMLButtonElement | null)[]>([]);
  const [popupTarget, setPopupTarget] = useState<HTMLElement | null>(null);
  const [query, setQuery] = useState("");
  const [batch, setBatch] = useState(50);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [loadedOptions, setLoadedOptions] = useState<readonly MultiSelectOption[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchError, setSearchError] = useState("");
  const updateQuery = (next: string) => {
    setQuery(next);
    if (loadOptions) {
      setLoadedOptions([]);
      setSearchError("");
      setLoading(true);
    }
  };
  useEffect(() => {
    if (!loadOptions) return;
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true);
      setSearchError("");
      void loadOptions(query)
        .then(result => { if (active) setLoadedOptions(result); })
        .catch(() => { if (active) { setLoadedOptions([]); setSearchError("Suggestions unavailable; custom values still work."); } })
        .finally(() => { if (active) setLoading(false); });
    }, query ? 120 : 0);
    return () => { active = false; window.clearTimeout(timer); };
  }, [loadOptions, query]);
  const availableOptions = loadOptions ? loadedOptions : options;
  const normalized = useMemo(() => availableOptions.map(option => ({
    option, label: normalize(option.label), text: normalize(`${option.label} ${option.value} ${option.searchText ?? ""}`),
  })), [availableOptions]);
  const tokens = normalize(query).split(/\s+/).filter(Boolean);
  const matches = loadOptions ? [...availableOptions] : normalized.filter(item => tokens.every(token => item.text.includes(token))).sort((a, b) => {
    const queryKey = normalize(query);
    const rankA = a.label === queryKey ? 0 : a.label.startsWith(queryKey) ? 1 : 2;
    const rankB = b.label === queryKey ? 0 : b.label.startsWith(queryKey) ? 1 : 2;
    return rankA - rankB || a.option.label.localeCompare(b.option.label);
  }).map(item => item.option);
  const selectedKeys = new Set(value.map(normalize));
  const custom = query.trim() && allowCustom && !loading && !normalized.some(item => item.label === normalize(query)) && !selectedKeys.has(normalize(query))
    ? query.trim() : "";
  const choices = custom ? [...matches.slice(0, batch), { value: custom, label: `Use “${custom}”` }] : matches.slice(0, batch);
  const commit = (item: string) => {
    const next = selectedKeys.has(normalize(item))
      ? value.filter(current => normalize(current) !== normalize(item))
      : [...value, item];
    const seen = new Set<string>();
    onChange(next.filter(current => { const key = normalize(current); if (!key || seen.has(key)) return false; seen.add(key); return true; }));
    updateQuery(""); setBatch(50); setActiveIndex(-1); setOpen(false);
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (event.key === "ArrowDown") { event.preventDefault(); setOpen(true); setActiveIndex(index => (index + 1) % Math.max(choices.length, 1)); return; }
    if (event.key === "ArrowUp") { event.preventDefault(); setOpen(true); setActiveIndex(index => index <= 0 ? Math.max(choices.length - 1, 0) : index - 1); return; }
    if (event.key === "Enter") {
      event.preventDefault(); event.stopPropagation();
      if (open && activeIndex >= 0 && choices[activeIndex]) commit(choices[activeIndex].value);
      return;
    }
    if (event.key === "Escape" && open) { event.preventDefault(); event.stopPropagation(); setOpen(false); updateQuery(""); setActiveIndex(-1); return; }
    if (event.key === "Backspace" && !query && value.length) { event.preventDefault(); removers.current[value.length - 1]?.focus(); }
  };
  return <div className="grid min-w-0 gap-2">
    <span className="sr-only" role="status" aria-live="polite">{searchError || (loading ? `Searching ${label.toLocaleLowerCase()}…` : `${value.length} ${label} selections. Showing ${Math.min(batch, matches.length)} of ${matches.length} results.`)}</span>
    <PopupPosition root={root} open={open} onTarget={setPopupTarget}>
      <div ref={root} className="flex min-w-0 gap-2">
        <input ref={input} type="text" role="combobox" aria-label={label} aria-describedby={describedBy}
          aria-expanded={open} aria-autocomplete="list" aria-controls={`${label.replace(/\W+/g, "-")}-options`}
          aria-busy={loading}
          aria-activedescendant={open && activeIndex >= 0 ? `${label.replace(/\W+/g, "-")}-option-${activeIndex}` : undefined}
          autoComplete="off" value={query} disabled={disabled} placeholder={placeholder ?? `Search ${label.toLocaleLowerCase()}`}
          className="min-h-[var(--control-height)] min-w-0 flex-1 border border-[var(--color-ink)] bg-[var(--color-paper)] px-3 py-2 text-sm normal-case text-[var(--color-ink)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)] disabled:opacity-60"
          onFocus={() => setOpen(true)} onClick={() => setOpen(true)} onChange={event => { updateQuery(event.target.value); setBatch(50); setActiveIndex(-1); setOpen(true); }}
          onKeyDown={handleKeyDown} onBlur={() => { if (!root.current?.contains(document.activeElement)) setOpen(false); }} />
        <button type="button" aria-label={`Open ${label}`} disabled={disabled} aria-expanded={open}
          className="min-h-[var(--control-height)] min-w-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)] disabled:opacity-60"
          onMouseDown={event => event.preventDefault()} onClick={() => { setOpen(current => !current); input.current?.focus(); }}><Icon name="down" size="sm" /></button>
        {popupTarget && open && createPortal(<div data-select-popup className="fixed z-[100] border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] shadow-none" style={positionOptions(root.current)}>
          <ul id={`${label.replace(/\W+/g, "-")}-options`} role="listbox" aria-label={`${label} options`} aria-multiselectable="true" className="max-h-60 overflow-auto p-1 focus:outline-none">
            {choices.map((option, index) => <li id={`${label.replace(/\W+/g, "-")}-option-${index}`} key={`${option.value}:${index}`} role="option"
              aria-selected={selectedKeys.has(normalize(option.value))} onMouseDown={event => event.preventDefault()} onMouseEnter={() => setActiveIndex(index)} onClick={() => commit(option.value)}
              className={`flex cursor-pointer items-start gap-2 px-2 py-2 text-sm normal-case ${activeIndex === index ? "bg-[var(--color-paper-2)]" : ""}`}>
              <span aria-hidden="true" className="w-4 shrink-0">{selectedKeys.has(normalize(option.value)) ? "✓" : ""}</span>
              <span className="min-w-0 break-words"><span>{option.label}</span>{option.description && <span className="block text-xs text-[var(--color-ink-mute)]">{option.description}</span>}</span>
            </li>)}
            {!choices.length && <li className="px-2 py-2 text-sm">No matches</li>}
          </ul>
          {matches.length > batch && <button type="button" className="w-full min-h-9 border-t border-[var(--color-border)] px-2 text-left text-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)]" onMouseDown={event => event.preventDefault()} onClick={() => setBatch(current => current + 50)}>Show more results</button>}
        </div>, popupTarget)}
      </div>
    </PopupPosition>
    {!!value.length && <ul aria-label={`${label} selections`} className="grid max-h-40 min-w-0 gap-1 overflow-y-auto">
      {value.map((item, index) => <li key={`${index}:${item}`} className="flex min-w-0 items-center justify-between gap-2 border border-[var(--color-border)] bg-[var(--color-surface-raised)] pl-2 text-sm normal-case">
        <span className="min-w-0 break-words py-1">{item}</span><button type="button" ref={node => { removers.current[index] = node; }} aria-label={`Remove ${item} from ${label}`} className="min-h-8 min-w-8 shrink-0 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)]" onClick={() => { onChange(value.filter((_, i) => i !== index)); requestAnimationFrame(() => (removers.current[Math.max(0, index - 1)] || input.current)?.focus()); }}>×</button>
      </li>)}
    </ul>}
  </div>;
}
