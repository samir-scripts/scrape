import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import type { JobStatusEvent } from "../bindings/JobStatusEvent";
import type { Page } from "../bindings/Page";
import type { Settings } from "../bindings/Settings";
import type { Source } from "../bindings/Source";
import type { SourceCadence } from "../bindings/SourceCadence";
import type { TransientSourceInput } from "../bindings/TransientSourceInput";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import type { CronValidationResult } from "../bindings/CronValidationResult";
import type { SchedulerStatus } from "../bindings/SchedulerStatus";
import type { WorkerEvent } from "../bindings/WorkerEvent";
import type { JobFilter } from "../bindings/JobFilter";
import type { FilterImpactPreview } from "../bindings/FilterImpactPreview";
import type { LocationOptions } from "../bindings/LocationOptions";
import type { ParsedLocation } from "../bindings/ParsedLocation";
import type { ScrapeLogEntry } from "../bindings/ScrapeLogEntry";
import type { Metrics } from "../bindings/Metrics";
import type { AppNotification } from "../bindings/AppNotification";
import type { UpdateCheckResult } from "../bindings/UpdateCheckResult";

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
  system_time_format: "12h",
  system_date_format: "MM/DD/YYYY",
  ui_theme: "paper",
  ui_reduce_motion: false,
  ui_tooltips_enabled: true,
  ui_font_family: "default",
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
  app_auto_update: true,
  app_update_channel: "stable",
  app_last_update_check: null,
};

let devSettings: Settings = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_settings");
      if (saved) {
        const parsed = JSON.parse(saved) as Record<string, unknown>;
        const normalized: Record<string, unknown> = {};
        for (const [k, v] of Object.entries(parsed)) {
          normalized[k.replace(/\./g, "_")] = v;
          normalized[k] = v;
        }
        return { ...DEFAULT_SETTINGS, ...normalized };
      }
    }
  } catch {}
  return { ...DEFAULT_SETTINGS };
})();

let devNotifications: AppNotification[] = [
  {
    id: "mock-1",
    kind: "system",
    title: "SYSTEM READY",
    message: "SCRAPE initialized in local preview mode.",
    action_label: null,
    action_payload: null,
    read: false,
    created_at: new Date().toISOString(),
  },
];

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

  toggleJobFavorite: (id: string): Promise<Job> =>
    invoke<Job>("toggle_job_favorite", { id }),

  setJobFavorite: (id: string, favorited: boolean): Promise<Job> =>
    invoke<Job>("set_job_favorite", { id, favorited }),

  cleanupExpiredJobs: (sourceId?: string): Promise<number> => {
    if (!isTauri()) return Promise.resolve(0);
    return invoke<number>("cleanup_expired_jobs", { sourceId: sourceId || null });
  },

  // Settings
  getSettings: (): Promise<Settings> => {
    if (!isTauri()) return Promise.resolve({ ...devSettings });
    return invoke<Settings>("get_settings");
  },

  updateSetting: (key: string, value: unknown): Promise<Settings> => {
    if (!isTauri()) {
      const fieldKey = key.replace(/\./g, "_");
      (devSettings as unknown as Record<string, unknown>)[fieldKey] = value;
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
    cadence?: SourceCadence;
    cadence_cron?: string | null;
    retention_days?: bigint | number | null;
    retention_policy?: string | null;
    filters?: unknown;
  }) =>
    invoke<Source>("create_source", {
      input: {
        config: null,
        enabled: true,
        cadence: "global",
        cadence_cron: null,
        retention_days: 30,
        retention_policy: "untracked_only",
        filters: {},
        ...input,
      },
    }),

  updateSource: (input: {
    id: string;
    name?: string | null;
    identifier?: string | null;
    config?: unknown;
    enabled?: boolean | null;
    cadence?: SourceCadence | null;
    cadence_cron?: string | null;
    retention_days?: bigint | number | null;
    retention_policy?: string | null;
    filters?: unknown;
  }) =>
    invoke<Source>("update_source", {
      input: {
        name: null,
        identifier: null,
        config: null,
        enabled: null,
        cadence: null,
        cadence_cron: null,
        retention_days: null,
        retention_policy: null,
        filters: null,
        ...input,
      },
    }),

  deleteSource: (id: string) =>
    invoke<void>("delete_source", { id }),

  // Scraping & Scheduler
  runScrapeNow: () =>
    invoke<ScrapeRun>("run_scrape_now"),

  runSourceNow: (id: string) =>
    invoke<ScrapeRun>("run_source_now", { id }),

  listScrapeRuns: (limit?: number) => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<ScrapeRun[]>("list_scrape_runs", { limit });
  },

  listSourceRuns: (sourceId: string, limit?: number) => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<ScrapeRun[]>("list_source_runs", { sourceId, limit });
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

  testSourceTransient: (input: TransientSourceInput) =>
    invoke<WorkerEvent[]>("test_source_transient", { input }),

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

  getMetrics: (granularity: "week" | "month" = "week", periods: number = 12): Promise<Metrics> => {
    if (!isTauri()) {
      return Promise.resolve({
        totals: {
          scraped: 0,
          applied: 0,
          interviewing: 0,
          offers: 0,
          favorited: 0,
          response_rate: 0,
          median_salary: null,
          avg_salary: null,
        },
        series: [],
        by_role_type: [],
        salary_distribution: [],
        streak: { current: 0, longest: 0, last_applied_on: null },
      });
    }
    return invoke<Metrics>("get_metrics", { granularity, periods });
  },

  // Notifications & Updater
  listNotifications: (limit?: number): Promise<AppNotification[]> => {
    if (!isTauri()) {
      return Promise.resolve(limit ? devNotifications.slice(0, limit) : [...devNotifications]);
    }
    return invoke<AppNotification[]>("list_notifications", { limit: limit ?? null });
  },

  markNotificationRead: (id: string): Promise<AppNotification> => {
    if (!isTauri()) {
      const item = devNotifications.find((n) => n.id === id);
      if (item) item.read = true;
      return Promise.resolve(item || devNotifications[0]);
    }
    return invoke<AppNotification>("mark_notification_read", { id });
  },

  markAllNotificationsRead: (): Promise<void> => {
    if (!isTauri()) {
      devNotifications.forEach((n) => (n.read = true));
      return Promise.resolve();
    }
    return invoke<void>("mark_all_notifications_read");
  },

  clearAllNotifications: (): Promise<void> => {
    if (!isTauri()) {
      devNotifications = [];
      return Promise.resolve();
    }
    return invoke<void>("clear_all_notifications");
  },

  countUnreadNotifications: (): Promise<number> => {
    if (!isTauri()) {
      return Promise.resolve(devNotifications.filter((n) => !n.read).length);
    }
    return invoke<number>("count_unread_notifications");
  },

  checkAppUpdate: (): Promise<UpdateCheckResult> => {
    if (!isTauri()) {
      return Promise.resolve({
        available: false,
        current_version: "0.1.0",
        latest_version: null,
        release_notes: null,
        error: null,
      });
    }
    return invoke<UpdateCheckResult>("check_app_update");
  },

  installAppUpdate: (): Promise<void> => {
    if (!isTauri()) {
      return Promise.resolve();
    }
    return invoke<void>("install_app_update");
  },
};
