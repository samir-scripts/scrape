import { Combobox as HeadlessCombobox, ComboboxInput, ComboboxOption, ComboboxOptions, Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/react";
import { Icon } from "./Icon";
import { createPortal } from "react-dom";
import { useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, RefObject, ReactNode, KeyboardEventHandler } from "react";

interface SelectProps {
  "aria-label": string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
  options: { value: string; label: string; disabled?: boolean }[];
}

export function Select({ options, className = "", ...props }: SelectProps) {
  const root = useRef<HTMLDivElement>(null);
  const [popupTarget, setPopupTarget] = useState<HTMLElement | null>(null);
  const selected = options.find((option) => option.value === props.value);
  return (
    <Listbox value={props.value} onChange={props.onChange} disabled={props.disabled}>
      {({ open }) => <PopupPosition root={root} open={open} onTarget={setPopupTarget}>
        <div className="relative inline-block w-full text-sm">
          <ListboxButton aria-label={props["aria-label"]}
            className={`flex min-h-[var(--control-height-sm)] w-full items-center justify-between gap-3 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2.5 py-1.5 text-left text-xs font-mono uppercase text-[var(--color-ink)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-signal)] disabled:cursor-not-allowed disabled:opacity-60 ${className}`}>
            <span className="truncate">{selected?.label ?? props.value}</span><Icon name="down" size="sm" />
          </ListboxButton>
          {popupTarget && createPortal(
            <ListboxOptions portal={false} modal={false} data-select-popup
              className="fixed z-[100] max-h-60 overflow-auto border border-[var(--color-ink)] bg-[var(--color-paper)] p-1 text-[var(--color-ink)] shadow-none focus:outline-none"
              style={positionOptions(root.current)}>
              {options.map((option) => (
                <ListboxOption key={option.value} value={option.value} disabled={option.disabled}
                  className="flex cursor-pointer items-center justify-between gap-3 px-2 py-2 text-xs font-mono uppercase data-[focus]:bg-[var(--color-paper-2)] data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50">
                  {({ selected: isSelected }) => <><span>{option.label}</span>{isSelected && <Icon name="check" size="sm" />}</>}
                </ListboxOption>
              ))}
            </ListboxOptions>, popupTarget)}
        </div>
      </PopupPosition>}
    </Listbox>
  );
}

export function positionOptions(root: HTMLDivElement | null, minWidth = 0): CSSProperties {
  const button = root?.querySelector<HTMLElement>("button, input");
  if (!button) return { visibility: "hidden" };
  const rect = button.getBoundingClientRect();
  const width = Math.min(Math.max(rect.width, minWidth), window.innerWidth - 16);
  const scroll = root?.closest("dialog")?.querySelector<HTMLElement>("[data-dialog-scroll]")?.getBoundingClientRect();
  const below = Math.max(0, (scroll?.bottom ?? window.innerHeight) - rect.bottom - 8);
  const above = Math.max(0, rect.top - (scroll?.top ?? 8) - 8);
  const maxHeight = Math.min(240, window.innerHeight - 16, Math.max(below, above));
  const placeBelow = below >= Math.min(maxHeight, 160) || below >= above;
  const top = placeBelow ? rect.bottom + 2 : rect.top - maxHeight - 2;
  return { top, left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)), width, maxHeight };
}

export function PopupPosition({ root, open, onTarget, children }: {
  root: RefObject<HTMLDivElement | null>;
  open: boolean;
  onTarget: (target: HTMLElement) => void;
  children: ReactNode;
}) {
  useLayoutEffect(() => {
    onTarget(root.current?.closest("dialog") ?? document.body);
    if (!open) return;
    const update = () => {
      const anchor = root.current?.querySelector<HTMLElement>("button, input");
      const id = anchor?.getAttribute("aria-controls");
      const popup = id ? document.getElementById(id) : null;
      if (anchor && popup) Object.assign(popup.style, positionOptions(root.current, popup.getAttribute("role") === "menu" ? 192 : 0));
    };
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => { window.removeEventListener("resize", update); window.removeEventListener("scroll", update, true); };
  }, [root, open, onTarget]);
  return <div ref={root}>{children}</div>;
}

export function SearchCombobox({
  id, "aria-label": label, value, query, options, onChange, onQueryChange,
  onBlur, onFocus, onKeyDown, placeholder, disabled, invalid, describedBy, renderOption,
}: {
  "aria-label": string;
  id?: string;
  value: string;
  query: string;
  options: { value: string; label: string; disabled?: boolean; searchText?: string }[];
  onChange: (value: string) => void;
  onQueryChange: (query: string) => void;
  onBlur?: () => void;
  describedBy?: string;
  onFocus?: () => void;
  onKeyDown?: KeyboardEventHandler<HTMLInputElement>;
  placeholder?: string;
  disabled?: boolean;
  invalid?: boolean;
  renderOption?: (option: { value: string; label: string; disabled?: boolean; searchText?: string }) => ReactNode;
}) {
  const root = useRef<HTMLDivElement>(null);
  const [popupTarget, setPopupTarget] = useState<HTMLElement | null>(null);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const queryTokens = normalizedQuery.split(/\s+/).filter(Boolean);
  const matches = options.filter(option => {
    const searchTokens = (option.searchText ?? `${option.label} ${option.value}`)
      .toLocaleLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
    return queryTokens.every(query => searchTokens.some(token => token.startsWith(query)));
  });
  return <HeadlessCombobox value={value || null} onChange={selected => onChange(selected ?? "")} disabled={disabled}>
    {({ open }) => <PopupPosition root={root} open={open} onTarget={setPopupTarget}>
      <ComboboxInput id={id} aria-label={label} autoComplete="off" value={query}
        onChange={event => onQueryChange(event.target.value)} onBlur={onBlur} onKeyDown={onKeyDown} onFocus={onFocus}
        aria-invalid={invalid} aria-describedby={describedBy} placeholder={placeholder}
        className="normal-case w-full min-h-[var(--control-height)] px-3 py-2 text-sm bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:bg-[var(--color-paper-2)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] font-sans disabled:opacity-60" />
      {popupTarget && createPortal(
        <ComboboxOptions portal={false} modal={false} data-select-popup
          className="fixed z-[100] max-h-60 overflow-auto border border-[var(--color-ink)] bg-[var(--color-paper)] p-1 text-[var(--color-ink)] focus:outline-none"
          style={positionOptions(root.current)}>
          {matches.map(option => <ComboboxOption key={option.value} value={option.value} disabled={option.disabled}
            className="cursor-pointer px-2 py-2 text-xs data-[focus]:bg-[var(--color-paper-2)] data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50">{renderOption ? renderOption(option) : option.label}</ComboboxOption>)}
          {matches.length === 0 && <li className="px-2 py-2 text-xs text-[var(--color-ink-mute)]">No matches</li>}
        </ComboboxOptions>, popupTarget)}
    </PopupPosition>}
  </HeadlessCombobox>;
}
