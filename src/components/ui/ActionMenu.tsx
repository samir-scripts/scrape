import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/react";
import { createPortal } from "react-dom";
import { useRef, useState } from "react";
import type { ReactNode } from "react";
import { Icon } from "./Icon";
import { PopupPosition, positionOptions } from "./Select";

export interface ActionMenuItem {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  dividerBefore?: boolean;
  disabled?: boolean;
}

export function ActionMenu({ label, children, items }: {
  label: string;
  children?: ReactNode;
  items: ActionMenuItem[];
}) {
  const root = useRef<HTMLDivElement>(null);
  const [popupTarget, setPopupTarget] = useState<HTMLElement | null>(null);
  return <Menu>
    {({ open }) => <PopupPosition root={root} open={open} onTarget={setPopupTarget}>
      <MenuButton aria-label={label} title={label}
        className="inline-flex min-h-8 min-w-8 items-center justify-center border border-transparent px-2 text-[var(--color-ink)] hover:border-[var(--color-ink)]/50 hover:bg-[var(--color-paper-2)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)]">
        {children ?? <Icon name="more" />}
      </MenuButton>
      {popupTarget && createPortal(
        <MenuItems portal={false} data-select-popup
          className="fixed z-[100] min-w-48 overflow-auto border border-[var(--color-ink)] bg-[var(--color-paper)] p-1 text-[var(--color-ink)] focus:outline-none"
          style={positionOptions(root.current, 192)}>
          {items.map(item => <MenuItem key={item.label} disabled={item.disabled}>
            <button type="button" onClick={() => {
              item.onSelect();
              requestAnimationFrame(() => root.current?.querySelector("button")?.focus({ preventScroll: true }));
            }}
              className={`w-full px-3 py-2 text-left text-sm data-[focus]:bg-[var(--color-paper-2)] disabled:cursor-not-allowed disabled:opacity-50 ${item.dividerBefore ? "border-t border-[var(--color-ink)]/30 mt-1" : ""} ${item.danger ? "text-[var(--color-danger)]" : ""}`}>
              {item.label}
            </button>
          </MenuItem>)}
        </MenuItems>, popupTarget)}
    </PopupPosition>}
  </Menu>;
}
