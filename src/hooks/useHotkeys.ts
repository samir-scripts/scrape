import { useEffect, useRef } from "react";
import { useUIStore } from "../lib/store";
import { NAV_TABS } from "../lib/constants";

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
          setFilterPanelOpen(false);
          return;
        }
        if (drawerJobId) {
          setDrawerJobId(null);
          return;
        }
        if (view === "discovered" && selectedJobId) {
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
        const currentView = useUIStore.getState().view;
        const currentIdx = NAV_TABS.findIndex((t) => t.id === currentView);
        const prevIdx = (currentIdx - 1 + NAV_TABS.length) % NAV_TABS.length;
        useUIStore.getState().setView(NAV_TABS[prevIdx].id);
        return;
      }

      if (!e.shiftKey && (e.key === "e" || e.key === "E" || e.key === "]")) {
        e.preventDefault();
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
        searchInput?.select();
        return;
      }

      // Filter Drawer toggle: F
      if (!e.shiftKey && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
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
          optionsRef.current.onShiftLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "L" || e.key === "l") {
          e.preventDefault();
          optionsRef.current.onShiftRight?.();
          return;
        }
      }

      // Horizontal column navigation in Kanban: Left / H, Right / L (when Shift not held)
      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "h" || e.key === "H") {
          e.preventDefault();
          optionsRef.current.onColLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "l" || e.key === "L") {
          e.preventDefault();
          optionsRef.current.onColRight?.();
          return;
        }
      }

      // Item navigation: Down / J, Up / K
      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowDown" || e.key === "j" || e.key === "J") {
          e.preventDefault();
          optionsRef.current.onNextItem?.();
          return;
        }
        if (e.key === "ArrowUp" || e.key === "k" || e.key === "K") {
          e.preventDefault();
          optionsRef.current.onPrevItem?.();
          return;
        }
      }

      // Open URL: Enter
      if (!hasMod && e.key === "Enter") {
        e.preventDefault();
        optionsRef.current.onOpenUrl?.();
        return;
      }

      // Actions: A (Apply), X (Ignore), V (Viewed), U (Undo)
      if (!hasMod && !e.shiftKey) {
        if (e.key === "a" || e.key === "A") {
          e.preventDefault();
          optionsRef.current.onApply?.();
          return;
        }

        if (e.key === "x" || e.key === "X") {
          e.preventDefault();
          optionsRef.current.onIgnore?.();
          return;
        }

        if (e.key === "v" || e.key === "V") {
          e.preventDefault();
          optionsRef.current.onViewed?.();
          return;
        }

        if (e.key === "u" || e.key === "U") {
          e.preventDefault();
          optionsRef.current.onUndo?.();
          return;
        }

        if (e.key === "s" || e.key === "S") {
          e.preventDefault();
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
          optionsRef.current.onDelete?.();
          return;
        }
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
