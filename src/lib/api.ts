import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import type { JobStatusEvent } from "../bindings/JobStatusEvent";
import type { Page } from "../bindings/Page";
import type { Settings } from "../bindings/Settings";
import type { Source } from "../bindings/Source";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import type { CronValidationResult } from "../bindings/CronValidationResult";
import type { SchedulerStatus } from "../bindings/SchedulerStatus";
import type { WorkerEvent } from "../bindings/WorkerEvent";
import type { JobFilter } from "../bindings/JobFilter";
import type { FilterImpactPreview } from "../bindings/FilterImpactPreview";
import type { LocationOptions } from "../bindings/LocationOptions";
import type { ParsedLocation } from "../bindings/ParsedLocation";
import type { ScrapeLogEntry } from "../bindings/ScrapeLogEntry";

export interface DetectResult {
  kind: string;
  identifier: string;
  name: string;
}

export type ApiJobFilter = Partial<JobFilter>;

export const DEFAULT_SETTINGS: Settings = {
  system_locale: "en-US",
  system_timezone: "America/New_York",
  system_week_start: "sunday",
  ui_theme: "paper",
  ui_reduce_motion: false,
  scraper_enabled: true,
  scraper_cron: "0 0 */6 * * *",
  scraper_run_on_startup: false,
  filters_include_keywords: [],
  filters_exclude_keywords: [],
  filters_locations: [],
  filters_role_types: [],
  notifications_enabled: true,
  app_start_hidden: false,
  app_launch_at_login: false,
};

let devSettings: Settings = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_settings");
      if (saved) return JSON.parse(saved);
    }
  } catch {}
  return { ...DEFAULT_SETTINGS };
})();

export const api = {
  // Jobs
  listJobs: (filter?: ApiJobFilter): Promise<Page<Job>> => {
    if (!isTauri()) {
      return Promise.resolve({
        items: [],
        total: BigInt(0),
        limit: BigInt(2000),
        offset: BigInt(0),
      });
    }
    return invoke<Page<Job>>("list_jobs", { filter: filter || null });
  },

  getJob: (id: string) =>
    invoke<Job>("get_job", { id }),

  updateJobStatus: (id: string, status: JobStatus) =>
    invoke<Job>("update_job_status", { id, status }),

  bulkUpdateJobStatus: (ids: string[], status: JobStatus) =>
    invoke<number>("bulk_update_job_status", { ids, status }),

  updateJobNotes: (id: string, notes?: string) =>
    invoke<Job>("update_job_notes", { id, notes: notes || null }),

  getJobStatusEvents: (jobId: string) => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<JobStatusEvent[]>("get_job_status_events", { jobId });
  },

  // Settings
  getSettings: (): Promise<Settings> => {
    if (!isTauri()) return Promise.resolve({ ...devSettings });
    return invoke<Settings>("get_settings");
  },

  updateSetting: (key: string, value: unknown): Promise<Settings> => {
    if (!isTauri()) {
      (devSettings as unknown as Record<string, unknown>)[key] = value;
      try {
        if (typeof localStorage !== "undefined") {
          localStorage.setItem("scrape:dev_settings", JSON.stringify(devSettings));
        }
      } catch {}
      return Promise.resolve({ ...devSettings });
    }
    return invoke<Settings>("update_setting", { key, value });
  },

  // Sources
  listSources: (): Promise<Source[]> => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<Source[]>("list_sources");
  },

  getSource: (id: string) =>
    invoke<Source>("get_source", { id }),

  createSource: (input: {
    kind: string;
    name: string;
    identifier: string;
    config?: unknown;
    enabled?: boolean;
  }) =>
    invoke<Source>("create_source", {
      input: {
        config: null,
        enabled: true,
        ...input,
      },
    }),

  updateSource: (input: {
    id: string;
    name?: string | null;
    identifier?: string | null;
    config?: unknown;
    enabled?: boolean | null;
  }) =>
    invoke<Source>("update_source", {
      input: {
        name: null,
        identifier: null,
        config: null,
        enabled: null,
        ...input,
      },
    }),

  deleteSource: (id: string) =>
    invoke<void>("delete_source", { id }),

  // Scraping & Scheduler
  runScrapeNow: () =>
    invoke<ScrapeRun>("run_scrape_now"),

  listScrapeRuns: (limit?: number) => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<ScrapeRun[]>("list_scrape_runs", { limit });
  },

  validateCron: (expr: string): Promise<CronValidationResult> => {
    if (!isTauri()) return Promise.resolve({ ok: true, next_runs: ["2026-10-10T00:00:00Z"], error: null });
    return invoke<CronValidationResult>("validate_cron", { expr });
  },

  getSchedulerStatus: (): Promise<SchedulerStatus> => {
    if (!isTauri()) {
      return Promise.resolve({
        enabled: devSettings.scraper_enabled,
        next_run_at: null,
        running: false,
      });
    }
    return invoke<SchedulerStatus>("get_scheduler_status");
  },

  setSchedulerPaused: (paused: boolean) =>
    invoke<void>("set_scheduler_paused", { paused }),

  detectSource: (url: string) =>
    invoke<DetectResult>("detect_source", { url }),

  testSource: (id: string) =>
    invoke<WorkerEvent[]>("test_source", { id }),

  getJobLocations: (jobId: string): Promise<ParsedLocation[]> => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<ParsedLocation[]>("get_job_locations", { jobId });
  },

  reclassifyJobs: (): Promise<number> => {
    if (!isTauri()) return Promise.resolve(0);
    return invoke<number>("reclassify_jobs");
  },

  previewFilterImpact: (spec: JobFilter): Promise<FilterImpactPreview> => {
    if (!isTauri()) return Promise.resolve({ kept: BigInt(0), total: BigInt(0) });
    return invoke<FilterImpactPreview>("preview_filter_impact", { spec });
  },

  listLocationOptions: (): Promise<LocationOptions> => {
    if (!isTauri()) return Promise.resolve({ countries: [], regions: [] });
    return invoke<LocationOptions>("list_location_options");
  },

  getScrapeRun: (runId: string): Promise<ScrapeRun> => {
    return invoke<ScrapeRun>("get_scrape_run", { runId });
  },

  getScrapeLogs: (runId: string): Promise<ScrapeLogEntry[]> => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<ScrapeLogEntry[]>("get_scrape_logs", { runId });
  },
};
