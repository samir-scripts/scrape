import { useEffect } from "react";
import { useUIStore } from "../lib/store";
import { NAV_TABS } from "../lib/constants";

export interface HotkeyOptions {
  onNextItem?: () => void;
  onPrevItem?: () => void;
  onOpenUrl?: () => void;
  onApply?: () => void;
  onIgnore?: () => void;
  onViewed?: () => void;
  onUndo?: () => void;
  onShiftLeft?: () => void;
  onShiftRight?: () => void;
}

export function useHotkeys(options: HotkeyOptions = {}) {
  const { view, setView } = useUIStore();

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const activeEl = document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";

      if (e.key === "Escape") {
        if (isInput) {
          (activeEl as HTMLElement).blur();
          return;
        }
      }

      if (isInput) {
        return;
      }

      // Tab switching: Q / [ (prev), E / ] (next)
      if (e.key === "q" || e.key === "Q" || e.key === "[") {
        e.preventDefault();
        const currentIdx = NAV_TABS.findIndex((t) => t.id === view);
        const prevIdx = (currentIdx - 1 + NAV_TABS.length) % NAV_TABS.length;
        setView(NAV_TABS[prevIdx].id);
        return;
      }

      if (e.key === "e" || e.key === "E" || e.key === "]") {
        e.preventDefault();
        const currentIdx = NAV_TABS.findIndex((t) => t.id === view);
        const nextIdx = (currentIdx + 1) % NAV_TABS.length;
        setView(NAV_TABS[nextIdx].id);
        return;
      }

      // Search: /
      if (e.key === "/") {
        e.preventDefault();
        const searchInput = document.getElementById(
          "global-search-input"
        ) as HTMLInputElement | null;
        searchInput?.focus();
        searchInput?.select();
        return;
      }

      // Column shift in Tracked Kanban: Shift + Left / Right
      if (e.shiftKey && e.key === "ArrowLeft") {
        e.preventDefault();
        options.onShiftLeft?.();
        return;
      }

      if (e.shiftKey && e.key === "ArrowRight") {
        e.preventDefault();
        options.onShiftRight?.();
        return;
      }

      // Item navigation: Down / J, Up / K
      if (e.key === "ArrowDown" || e.key === "j" || e.key === "J") {
        e.preventDefault();
        options.onNextItem?.();
        return;
      }

      if (e.key === "ArrowUp" || e.key === "k" || e.key === "K") {
        e.preventDefault();
        options.onPrevItem?.();
        return;
      }

      // Open URL: Enter
      if (e.key === "Enter") {
        e.preventDefault();
        options.onOpenUrl?.();
        return;
      }

      // Actions: A (Apply), X (Ignore), V (Viewed), U (Undo)
      if (e.key === "a" || e.key === "A") {
        e.preventDefault();
        options.onApply?.();
        return;
      }

      if (e.key === "x" || e.key === "X") {
        e.preventDefault();
        options.onIgnore?.();
        return;
      }

      if (e.key === "v" || e.key === "V") {
        e.preventDefault();
        options.onViewed?.();
        return;
      }

      if (e.key === "u" || e.key === "U") {
        e.preventDefault();
        options.onUndo?.();
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [view, setView, options]);
}
