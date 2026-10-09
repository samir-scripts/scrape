import { invoke } from "@tauri-apps/api/core";
import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";
import type { JobStatus } from "../bindings/JobStatus";
import type { JobStatusEvent } from "../bindings/JobStatusEvent";
import type { Page } from "../bindings/Page";
import type { Settings } from "../bindings/Settings";
import type { Source } from "../bindings/Source";
import type { CreateSourceInput } from "../bindings/CreateSourceInput";
import type { UpdateSourceInput } from "../bindings/UpdateSourceInput";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import type { CronValidationResult } from "../bindings/CronValidationResult";
import type { SchedulerStatus } from "../bindings/SchedulerStatus";
import type { WorkerEvent } from "../bindings/WorkerEvent";

export interface DetectResult {
  kind: string;
  identifier: string;
  name: string;
}

export const api = {
  // Jobs
  listJobs: (filter?: JobFilter) =>
    invoke<Page<Job>>("list_jobs", { filter }),

  getJob: (id: string) =>
    invoke<Job>("get_job", { id }),

  updateJobStatus: (id: string, status: JobStatus) =>
    invoke<Job>("update_job_status", { id, status }),

  bulkUpdateJobStatus: (ids: string[], status: JobStatus) =>
    invoke<number>("bulk_update_job_status", { ids, status }),

  updateJobNotes: (id: string, notes?: string) =>
    invoke<Job>("update_job_notes", { id, notes }),

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

  createSource: (input: CreateSourceInput) =>
    invoke<Source>("create_source", { input }),

  updateSource: (input: UpdateSourceInput) =>
    invoke<Source>("update_source", { input }),

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
