import { create } from "zustand";
import type { JobStatus } from "../bindings/JobStatus";
import type { NavTabId } from "./constants";

import type { JobFilter } from "../bindings/JobFilter";

export type ScraperCategory =
  | "workflows"
  | "logs"
  | "sources"
  | "schedule"
  | "filters";

export type SettingsCategory =
  | "appearance"
  | "system"
  | "removed"
  | "data";

export interface UndoItem {
  jobId: string;
  previousStatus: JobStatus;
}

export const DEFAULT_FILTER_SPEC: Partial<JobFilter> = {
  search: null,
  favorited_only: null,
  salary_min: null,
  salary_max: null,
  salary_currency: null,
  salary_interval: null,
  salary_include_unknown: false,
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
  dataRevision: number;
  // Navigation
  view: NavTabId;
  setView: (view: NavTabId) => void;

  // Discovered View state
  discoveredViewMode: "panes" | "table";
  setDiscoveredViewMode: (mode: "panes" | "table") => void;
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
  clearFilters: () => void;

  // Tracked View state
  trackedViewMode: "kanban" | "list";
  setTrackedViewMode: (mode: "kanban" | "list") => void;
  drawerJobId: string | null;
  setDrawerJobId: (id: string | null) => void;

  // Scraper View state
  scraperCategory: ScraperCategory;
  setScraperCategory: (cat: ScraperCategory) => void;
  scraperMode: "simple" | "advanced";
  setScraperMode: (mode: "simple" | "advanced") => void;

  // Settings View state
  settingsCategory: SettingsCategory;
  setSettingsCategory: (cat: SettingsCategory) => void;

  // Global Tooltips Visibility
  tooltipsEnabled: boolean;
  setTooltipsEnabled: (enabled: boolean) => void;

  // Notification Popover state
  isNotificationOpen: boolean;
  setNotificationOpen: (open: boolean) => void;
  toggleNotificationOpen: () => void;
  unreadNotificationCount: number;
  setUnreadNotificationCount: (count: number) => void;

  // Global Scrape Status
  isScanning: boolean;
  scanStatusText: string;
  setScanning: (scanning: boolean, text?: string) => void;

  // Active Running Sources
  runningSourceIds: string[];
  setSourceRunning: (sourceId: string, isRunning: boolean) => void;
  clearRunningSources: () => void;

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
  dataRevision: 0,
  view: (getStored("scrape:view") as NavTabId) || "discovered",
  setView: (view) => {
    setStored("scrape:view", view);
    set({ view });
  },

  discoveredViewMode:
    (getStored("scrape:discoveredMode") as "panes" | "table") || "panes",
  setDiscoveredViewMode: (mode) => {
    setStored("scrape:discoveredMode", mode);
    set({ discoveredViewMode: mode });
  },

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

  filterSpec: (() => {
    try { const saved = JSON.parse(getStored("scrape:job_filters:v2") || "null");
      return saved && typeof saved === "object" && !Array.isArray(saved) ? { ...DEFAULT_FILTER_SPEC, ...saved } : { ...DEFAULT_FILTER_SPEC };
    } catch { return { ...DEFAULT_FILTER_SPEC }; }
  })(),
  setFilterSpec: (updater) => set((state) => {
    const filterSpec = typeof updater === "function" ? updater(state.filterSpec) : { ...state.filterSpec, ...updater };
    setStored("scrape:job_filters:v2", JSON.stringify(filterSpec));
    return { filterSpec };
  }),
  resetFilterSpec: () => {
    setStored("scrape:job_filters:v2", JSON.stringify(DEFAULT_FILTER_SPEC));
    set({ filterSpec: { ...DEFAULT_FILTER_SPEC } });
  },
  clearFilters: () => set((state) => {
    const filterSpec = { ...DEFAULT_FILTER_SPEC, sort: state.filterSpec.sort ?? null };
    setStored("scrape:job_filters:v2", JSON.stringify(filterSpec));
    return { filterSpec };
  }),

  trackedViewMode:
    (getStored("scrape:trackedMode") as "kanban" | "list") || "kanban",
  setTrackedViewMode: (mode) => {
    setStored("scrape:trackedMode", mode);
    set({ trackedViewMode: mode });
  },

  drawerJobId: null,
  setDrawerJobId: (drawerJobId) => set({ drawerJobId }),

  scraperCategory:
    ((getStored("scrape:scraperCategory") as ScraperCategory) === "logs" ? "logs" : "workflows"),
  setScraperCategory: (scraperCategory) => {
    setStored("scrape:scraperCategory", scraperCategory);
    set({ scraperCategory });
  },

  scraperMode:
    ((getStored("scrape:scraperMode") as "simple" | "advanced") === "advanced" ? "advanced" : "simple"),
  setScraperMode: (scraperMode) => {
    setStored("scrape:scraperMode", scraperMode);
    set({ scraperMode });
  },

  settingsCategory:
    (getStored("scrape:settingsCategory") as SettingsCategory) || "appearance",
  setSettingsCategory: (settingsCategory) => {
    setStored("scrape:settingsCategory", settingsCategory);
    set({ settingsCategory });
  },

  tooltipsEnabled: getStored("scrape:tooltipsEnabled") !== "false",
  setTooltipsEnabled: (tooltipsEnabled) => {
    setStored("scrape:tooltipsEnabled", String(tooltipsEnabled));
    set({ tooltipsEnabled });
  },

  isNotificationOpen: false,
  setNotificationOpen: (open) => set({ isNotificationOpen: open }),
  toggleNotificationOpen: () =>
    set((state) => ({ isNotificationOpen: !state.isNotificationOpen })),
  unreadNotificationCount: 0,
  setUnreadNotificationCount: (count) => set({ unreadNotificationCount: count }),

  isScanning: false,
  scanStatusText: "",
  setScanning: (isScanning, scanStatusText = "") =>
    set({ isScanning, scanStatusText }),

  runningSourceIds: [],
  setSourceRunning: (sourceId, isRunning) =>
    set((state) => ({
      runningSourceIds: isRunning
        ? state.runningSourceIds.includes(sourceId)
          ? state.runningSourceIds
          : [...state.runningSourceIds, sourceId]
        : state.runningSourceIds.filter((id) => id !== sourceId),
    })),
  clearRunningSources: () => set({ runningSourceIds: [] }),

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
