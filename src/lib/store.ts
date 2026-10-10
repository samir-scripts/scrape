import { create } from "zustand";
import type { JobStatus } from "../bindings/JobStatus";
import type { NavTabId } from "./constants";

import type { JobFilter } from "../bindings/JobFilter";

export type ScraperCategory =
  | "sources"
  | "schedule"
  | "filters"
  | "logs";

export type SettingsCategory =
  | "appearance"
  | "system";

export interface UndoItem {
  jobId: string;
  previousStatus: JobStatus;
}

export const DEFAULT_FILTER_SPEC: Partial<JobFilter> = {
  salary_min: null,
  salary_max: null,
  salary_currency: null,
  salary_include_unknown: true,
  experience_min: null,
  experience_max: null,
  experience_include_unknown: true,
  new_grad: "any",
  sponsorship: null,
  clearance: null,
  countries: null,
  regions: null,
  remote: null,
  location_include_unknown: true,
  role_types: null,
};

interface UIState {
  // Navigation
  view: NavTabId;
  setView: (view: NavTabId) => void;

  // Discovered View state
  roleTypeFilter: string | null;
  setRoleTypeFilter: (filter: string | null) => void;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  selectedJobId: string | null;
  setSelectedJobId: (id: string | null) => void;
  selectedJobIds: string[];
  toggleJobSelection: (id: string) => void;
  clearJobSelection: () => void;

  // Filter Panel state
  isFilterPanelOpen: boolean;
  setFilterPanelOpen: (open: boolean) => void;
  toggleFilterPanel: () => void;
  filterSpec: Partial<JobFilter>;
  setFilterSpec: (updater: Partial<JobFilter> | ((prev: Partial<JobFilter>) => Partial<JobFilter>)) => void;
  resetFilterSpec: () => void;

  // Tracked View state
  trackedViewMode: "kanban" | "list";
  setTrackedViewMode: (mode: "kanban" | "list") => void;
  drawerJobId: string | null;
  setDrawerJobId: (id: string | null) => void;

  // Scraper View state
  scraperCategory: ScraperCategory;
  setScraperCategory: (cat: ScraperCategory) => void;

  // Settings View state
  settingsCategory: SettingsCategory;
  setSettingsCategory: (cat: SettingsCategory) => void;

  // Global Scrape Status
  isScanning: boolean;
  scanStatusText: string;
  setScanning: (scanning: boolean, text?: string) => void;

  // Undo history for status transitions
  undoStack: UndoItem[];
  pushUndo: (item: UndoItem) => void;
  popUndo: () => UndoItem | undefined;
}

const getStored = (key: string): string | null => {
  try {
    if (typeof window !== "undefined" && window.localStorage) {
      return window.localStorage.getItem(key);
    }
  } catch {
    // Ignore in non-browser/restricted environments
  }
  return null;
};

const setStored = (key: string, value: string): void => {
  try {
    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(key, value);
    }
  } catch {
    // Ignore in non-browser/restricted environments
  }
};

export const useUIStore = create<UIState>((set, get) => ({
  view: (getStored("scrape:view") as NavTabId) || "discovered",
  setView: (view) => {
    setStored("scrape:view", view);
    set({ view });
  },

  roleTypeFilter: null,
  setRoleTypeFilter: (roleTypeFilter) => set({ roleTypeFilter }),

  searchQuery: "",
  setSearchQuery: (searchQuery) => set({ searchQuery }),

  selectedJobId: null,
  setSelectedJobId: (selectedJobId) => set({ selectedJobId }),

  selectedJobIds: [],
  toggleJobSelection: (id) =>
    set((state) => {
      const exists = state.selectedJobIds.includes(id);
      return {
        selectedJobIds: exists
          ? state.selectedJobIds.filter((item) => item !== id)
          : [...state.selectedJobIds, id],
      };
    }),
  clearJobSelection: () => set({ selectedJobIds: [] }),

  isFilterPanelOpen: false,
  setFilterPanelOpen: (open) => set({ isFilterPanelOpen: open }),
  toggleFilterPanel: () =>
    set((state) => ({ isFilterPanelOpen: !state.isFilterPanelOpen })),

  filterSpec: { ...DEFAULT_FILTER_SPEC },
  setFilterSpec: (updater) =>
    set((state) => ({
      filterSpec:
        typeof updater === "function"
          ? updater(state.filterSpec)
          : { ...state.filterSpec, ...updater },
    })),
  resetFilterSpec: () => set({ filterSpec: { ...DEFAULT_FILTER_SPEC } }),

  trackedViewMode:
    (getStored("scrape:trackedMode") as "kanban" | "list") || "kanban",
  setTrackedViewMode: (mode) => {
    setStored("scrape:trackedMode", mode);
    set({ trackedViewMode: mode });
  },

  drawerJobId: null,
  setDrawerJobId: (drawerJobId) => set({ drawerJobId }),

  scraperCategory:
    (getStored("scrape:scraperCategory") as ScraperCategory) || "sources",
  setScraperCategory: (scraperCategory) => {
    setStored("scrape:scraperCategory", scraperCategory);
    set({ scraperCategory });
  },

  settingsCategory:
    (getStored("scrape:settingsCategory") as SettingsCategory) || "appearance",
  setSettingsCategory: (settingsCategory) => {
    setStored("scrape:settingsCategory", settingsCategory);
    set({ settingsCategory });
  },

  isScanning: false,
  scanStatusText: "",
  setScanning: (isScanning, scanStatusText = "") =>
    set({ isScanning, scanStatusText }),

  undoStack: [],
  pushUndo: (item) =>
    set((state) => ({
      undoStack: [item, ...state.undoStack.slice(0, 19)],
    })),
  popUndo: () => {
    const stack = get().undoStack;
    if (stack.length === 0) return undefined;
    const [top, ...rest] = stack;
    set({ undoStack: rest });
    return top;
  },
}));
