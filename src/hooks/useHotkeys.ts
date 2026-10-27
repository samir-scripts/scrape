import { useEffect, useRef } from "react";
import { useUIStore } from "../lib/store";
import { NAV_TABS } from "../lib/constants";
import { recordControlAction } from "../lib/controlActivity";

/**
 * Root-level application hotkeys.
 * Mounted at the top level in App.tsx so navigation is active across all views:
 * - Tab switching: Q / [ (prev), E / ] (next)
 * - Global search: /
 * - Filter drawer toggle: F
 * - Escape: blur input, close drawer/filter panel
 */
export function useGlobalHotkeys() {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.defaultPrevented) return;
      const target = e.target instanceof Element ? e.target : document.activeElement;
      if (e.key !== "Escape" && target?.closest("button, a, input, textarea, select, [contenteditable], [role='button'], [role='switch'], [role='checkbox']")) return;
      const activeEl = document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";

      if (e.key === "Escape") {
        if (isInput) {
          recordControlAction("shortcut.blur", "shortcut");
          (activeEl as HTMLElement).blur();
          return;
        }
        const {
          isFilterPanelOpen,
          setFilterPanelOpen,
          drawerJobId,
          setDrawerJobId,
          view,
          selectedJobId,
          setSelectedJobId,
        } = useUIStore.getState();
        if (isFilterPanelOpen) {
          recordControlAction("shortcut.filter.close", "shortcut");
          setFilterPanelOpen(false);
          return;
        }
        if (drawerJobId) {
          recordControlAction("shortcut.drawer.close", "shortcut");
          setDrawerJobId(null);
          return;
        }
        if (view === "discovered" && selectedJobId) {
          recordControlAction("shortcut.selection.clear", "shortcut");
          setSelectedJobId(null);
          return;
        }
        return;
      }

      if (isInput) {
        return;
      }

      const hasMod = e.ctrlKey || e.metaKey || e.altKey;
      if (hasMod) return;

      // Tab switching: Q / [ (prev), E / ] (next)
      if (!e.shiftKey && (e.key === "q" || e.key === "Q" || e.key === "[")) {
        e.preventDefault();
        recordControlAction("shortcut.navigation.previous", "shortcut");
        const currentView = useUIStore.getState().view;
        const currentIdx = NAV_TABS.findIndex((t) => t.id === currentView);
        const prevIdx = (currentIdx - 1 + NAV_TABS.length) % NAV_TABS.length;
        useUIStore.getState().setView(NAV_TABS[prevIdx].id);
        return;
      }

      if (!e.shiftKey && (e.key === "e" || e.key === "E" || e.key === "]")) {
        e.preventDefault();
        recordControlAction("shortcut.navigation.next", "shortcut");
        const currentView = useUIStore.getState().view;
        const currentIdx = NAV_TABS.findIndex((t) => t.id === currentView);
        const nextIdx = (currentIdx + 1) % NAV_TABS.length;
        useUIStore.getState().setView(NAV_TABS[nextIdx].id);
        return;
      }

      // Search: /
      if (e.key === "/") {
        e.preventDefault();
        const searchInput = document.getElementById(
          "global-search-input"
        ) as HTMLInputElement | null;
        searchInput?.focus();
        if (searchInput) recordControlAction("shortcut.search", "shortcut");
        searchInput?.select();
        return;
      }

      // Filter Drawer toggle: F
      if (!e.shiftKey && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        recordControlAction("shortcut.filter.toggle", "shortcut");
        useUIStore.getState().toggleFilterPanel();
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}

export interface HotkeyOptions {
  onNextItem?: () => void;
  onPrevItem?: () => void;
  onColLeft?: () => void;
  onColRight?: () => void;
  onOpenUrl?: () => void;
  onApply?: () => void;
  onIgnore?: () => void;
  onViewed?: () => void;
  onUndo?: () => void;
  onToggleFavorite?: () => void;
  onShiftLeft?: () => void;
  onShiftRight?: () => void;
  onDelete?: () => void;
}

/**
 * View-specific hotkeys for Discovered and Tracked views.
 * Handles in-view list/grid navigation, stage shifts, and job status mutations.
 */
export function useHotkeys(options: HotkeyOptions = {}) {
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.defaultPrevented) return;
      const target = e.target instanceof Element ? e.target : document.activeElement;
      if (e.key !== "Escape" && target?.closest("button, a, input, textarea, select, [contenteditable], [role='button'], [role='switch'], [role='checkbox']")) return;
      const activeEl = document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";

      if (isInput) {
        return;
      }

      const hasMod = e.ctrlKey || e.metaKey || e.altKey;

      // Column shift in Tracked Kanban: Shift + Left / Right OR Shift + H / L
      if (!hasMod && e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "H" || e.key === "h") {
          e.preventDefault();
          if (optionsRef.current.onShiftLeft) recordControlAction("shortcut.stage.previous", "shortcut");
          optionsRef.current.onShiftLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "L" || e.key === "l") {
          e.preventDefault();
          if (optionsRef.current.onShiftRight) recordControlAction("shortcut.stage.next", "shortcut");
          optionsRef.current.onShiftRight?.();
          return;
        }
      }

      // Horizontal column navigation in Kanban: Left / H, Right / L (when Shift not held)
      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "h" || e.key === "H") {
          e.preventDefault();
          if (optionsRef.current.onColLeft) recordControlAction("shortcut.column.previous", "shortcut");
          optionsRef.current.onColLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "l" || e.key === "L") {
          e.preventDefault();
          if (optionsRef.current.onColRight) recordControlAction("shortcut.column.next", "shortcut");
          optionsRef.current.onColRight?.();
          return;
        }
      }

      // Item navigation: Down / J, Up / K
      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowDown" || e.key === "j" || e.key === "J") {
          e.preventDefault();
          if (optionsRef.current.onNextItem) recordControlAction("shortcut.item.next", "shortcut");
          optionsRef.current.onNextItem?.();
          return;
        }
        if (e.key === "ArrowUp" || e.key === "k" || e.key === "K") {
          e.preventDefault();
          if (optionsRef.current.onPrevItem) recordControlAction("shortcut.item.previous", "shortcut");
          optionsRef.current.onPrevItem?.();
          return;
        }
      }

      // Open URL: Enter
      if (!hasMod && e.key === "Enter") {
        e.preventDefault();
        if (optionsRef.current.onOpenUrl) recordControlAction("shortcut.job.open", "shortcut");
        optionsRef.current.onOpenUrl?.();
        return;
      }

      // Actions: A (Apply), X (Ignore), V (Viewed), U (Undo)
      if (!hasMod && !e.shiftKey) {
        if (e.key === "a" || e.key === "A") {
          e.preventDefault();
          if (optionsRef.current.onApply) recordControlAction("shortcut.job.apply", "shortcut");
          optionsRef.current.onApply?.();
          return;
        }

        if (e.key === "x" || e.key === "X") {
          e.preventDefault();
          if (optionsRef.current.onIgnore) recordControlAction("shortcut.job.ignore", "shortcut");
          optionsRef.current.onIgnore?.();
          return;
        }

        if (e.key === "v" || e.key === "V") {
          e.preventDefault();
          if (optionsRef.current.onViewed) recordControlAction("shortcut.job.viewed", "shortcut");
          optionsRef.current.onViewed?.();
          return;
        }

        if (e.key === "u" || e.key === "U") {
          e.preventDefault();
          if (optionsRef.current.onUndo) recordControlAction("shortcut.undo", "shortcut");
          optionsRef.current.onUndo?.();
          return;
        }

        if (e.key === "s" || e.key === "S") {
          e.preventDefault();
          if (optionsRef.current.onToggleFavorite) recordControlAction("shortcut.job.favorite", "shortcut");
          optionsRef.current.onToggleFavorite?.();
          return;
        }

        if (
          e.key === "Delete" ||
          e.key === "Backspace" ||
          e.key === "d" ||
          e.key === "D"
        ) {
          e.preventDefault();
          if (optionsRef.current.onDelete) recordControlAction("shortcut.job.delete", "shortcut");
          optionsRef.current.onDelete?.();
          return;
        }
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
