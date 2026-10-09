import { invoke } from "@tauri-apps/api/core";
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

import type { JobSort } from "../bindings/JobSort";
import type { RoleType } from "../bindings/RoleType";

export interface DetectResult {
  kind: string;
  identifier: string;
  name: string;
}

export interface ApiJobFilter {
  statuses?: JobStatus[] | null;
  role_types?: RoleType[] | null;
  search?: string | null;
  source_id?: string | null;
  sort?: JobSort | null;
  limit?: number | bigint | null;
  offset?: number | bigint | null;
}

export const api = {
  // Jobs
  listJobs: (filter?: ApiJobFilter) =>
    invoke<Page<Job>>("list_jobs", { filter: filter || null }),

  getJob: (id: string) =>
    invoke<Job>("get_job", { id }),

  updateJobStatus: (id: string, status: JobStatus) =>
    invoke<Job>("update_job_status", { id, status }),

  bulkUpdateJobStatus: (ids: string[], status: JobStatus) =>
    invoke<number>("bulk_update_job_status", { ids, status }),

  updateJobNotes: (id: string, notes?: string) =>
    invoke<Job>("update_job_notes", { id, notes: notes || null }),

  getJobStatusEvents: (jobId: string) =>
    invoke<JobStatusEvent[]>("get_job_status_events", { jobId }),

  // Settings
  getSettings: () =>
    invoke<Settings>("get_settings"),

  updateSetting: (key: string, value: unknown) =>
    invoke<Settings>("update_setting", { key, value }),

  // Sources
  listSources: () =>
    invoke<Source[]>("list_sources"),

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

  listScrapeRuns: (limit?: number) =>
    invoke<ScrapeRun[]>("list_scrape_runs", { limit }),

  validateCron: (expr: string) =>
    invoke<CronValidationResult>("validate_cron", { expr }),

  getSchedulerStatus: () =>
    invoke<SchedulerStatus>("get_scheduler_status"),

  setSchedulerPaused: (paused: boolean) =>
    invoke<void>("set_scheduler_paused", { paused }),

  detectSource: (url: string) =>
    invoke<DetectResult>("detect_source", { url }),

  testSource: (id: string) =>
    invoke<WorkerEvent[]>("test_source", { id }),
};
