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
import type { CreateNotificationInput } from "../bindings/CreateNotificationInput";
import type { UpdateCheckResult } from "../bindings/UpdateCheckResult";
import type { WorkplaceType } from "../bindings/WorkplaceType";

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
  app_confirm_job_deletion: true,
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

export const SAMPLE_SWE_SOURCE: Source = {
  id: "sample-swe-internships-1",
  kind: "greenhouse",
  name: "Cloudflare — Software Engineering Internships",
  identifier: "cloudflare",
  config: { url: "https://boards.greenhouse.io/cloudflare" },
  enabled: true,
  last_run_at: null,
  last_status: null,
  last_error: null,
  created_at: new Date().toISOString(),
  cadence: "daily",
  cadence_cron: "0 0 9 * * *",
  next_run_at: null,
  last_run_duration_ms: null,
  retention_days: BigInt(45),
  retention_policy: "untracked_only",
  filters: {
    role_types: ["internship"],
    include_keywords: ["Software", "Engineer", "Intern"],
    exclude_keywords: ["MBA", "Sales", "Marketing"],
    locations: ["Remote", "San Francisco, CA", "Austin, TX"],
  },
};

let devSources: Source[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_sources");
      if (saved) return JSON.parse(saved);
    }
  } catch {}
  return [{ ...SAMPLE_SWE_SOURCE }];
})();

function saveDevSources() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("scrape:dev_sources", JSON.stringify(devSources));
    }
  } catch {}
}

let devJobs: Job[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_jobs");
      if (saved) return JSON.parse(saved);
    }
  } catch {}
  return [];
})();

function saveDevJobs() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("scrape:dev_jobs", JSON.stringify(devJobs));
    }
  } catch {}
}

let devSourceRuns: ScrapeRun[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_runs");
      if (saved) return JSON.parse(saved);
    }
  } catch {}
  return [];
})();

function saveDevSourceRuns() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("scrape:dev_runs", JSON.stringify(devSourceRuns));
    }
  } catch {}
}

let devLogs: Record<string, ScrapeLogEntry[]> = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_logs");
      if (saved) return JSON.parse(saved);
    }
  } catch {}
  return {};
})();

function saveDevLogs() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("scrape:dev_logs", JSON.stringify(devLogs));
    }
  } catch {}
}

let devDeletedJobKeys: Set<string> = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("scrape:dev_deleted_jobs");
      if (saved) return new Set(JSON.parse(saved));
    }
  } catch {}
  return new Set();
})();

function saveDevDeletedJobKeys() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("scrape:dev_deleted_jobs", JSON.stringify([...devDeletedJobKeys]));
    }
  } catch {}
}

const executionDelayStep = () => {
  if (typeof process !== "undefined" && process.env?.NODE_ENV === "test") {
    return 15;
  }
  return 500;
};

export function __resetDevData() {
  devSources = [{ ...SAMPLE_SWE_SOURCE }];
  devJobs = [];
  devSourceRuns = [];
  devLogs = {};
  devDeletedJobKeys.clear();
  saveDevSources();
  saveDevJobs();
  saveDevSourceRuns();
  saveDevLogs();
  saveDevDeletedJobKeys();
}

export function __seedDevJobs(jobs: Job[]) {
  devJobs = [...jobs];
  saveDevJobs();
}

export function generateScrapedJobsForSource(source: Source): Job[] {
  const now = new Date().toISOString();
  const sourceName = source.name || "Target Company";
  const company =
    sourceName.split(" — ")[0].split(" - ")[0].trim() ||
    source.identifier ||
    "Cloudflare";
  const isCloudflare = company.toLowerCase().includes("cloudflare");

  const rawSampleRoles = isCloudflare
    ? [
        {
          external_id: "8199958",
          title: "Software Engineer Intern (2027) - Austin, TX",
          location: "Austin, TX",
          salary_min: 50,
          salary_max: 60,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8245197",
          title: "Software Engineer Intern (2027)",
          location: "Lisbon, Portugal",
          salary_min: 50,
          salary_max: 60,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8245211",
          title: "Software Engineer Intern (2027)",
          location: "London, UK",
          salary_min: 52,
          salary_max: 62,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8187404",
          title: "Systems Engineer, Network Protocols & Distributed Systems",
          location: "Austin, TX",
          salary_min: 55,
          salary_max: 65,
          role_type: "internship" as const,
          workplace_type: "hybrid" as const,
        },
        // Real Greenhouse postings from Cloudflare that exercise policy exclusion:
        {
          external_id: "8247492",
          title: "Technical Product Marketing Manager",
          location: "Remote",
          salary_min: 140000,
          salary_max: 180000,
          role_type: "full_time" as const,
          workplace_type: "remote" as const,
        },
        {
          external_id: "7695702",
          title: "Account Executive, FedCiv",
          location: "Washington DC, US",
          salary_min: 130000,
          salary_max: 170000,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "8097321",
          title: "AI Security Research & Red Team Engineer",
          location: "San Francisco, CA",
          salary_min: 165000,
          salary_max: 215000,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "7742773",
          title: "Systems Engineer, MAPS",
          location: "Austin, TX",
          salary_min: 150000,
          salary_max: 190000,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
      ]
    : [
        {
          external_id: "101",
          title: `${company} Software Engineer - Backend Systems`,
          location: "San Francisco, CA",
          salary_min: 145000,
          salary_max: 185000,
          role_type: ((source.filters as any)?.role_types?.[0] as any) || "full_time",
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "102",
          title: `Infrastructure Engineer - Platform & Reliability`,
          location: "Remote",
          salary_min: 155000,
          salary_max: 195000,
          role_type: ((source.filters as any)?.role_types?.[0] as any) || "full_time",
          workplace_type: "remote" as const,
        },
        {
          external_id: "103",
          title: `Full Stack Engineer - Product Experience`,
          location: "New York, NY",
          salary_min: 140000,
          salary_max: 175000,
          role_type: ((source.filters as any)?.role_types?.[0] as any) || "full_time",
          workplace_type: "onsite" as const,
        },
        {
          external_id: "104",
          title: `Systems Engineer Intern - Distributed Data`,
          location: "Remote",
          salary_min: 45,
          salary_max: 55,
          role_type: ((source.filters as any)?.role_types?.[0] as any) || "internship",
          workplace_type: "remote" as const,
        },
      ];

  const prefix = source.identifier || "src";
  const companySlug = company.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const companyIconUrl = `https://www.google.com/s2/favicons?domain=${
    companySlug.includes("cloudflare")
      ? "cloudflare.com"
      : companySlug.includes("stripe")
      ? "stripe.com"
      : `${companySlug}.com`
  }&sz=64`;

  const candidates: Job[] = rawSampleRoles.map((role, idx) => {
    const activeExternalId = (role as any).external_id || `${8097320 + idx}`;
    const jobId = `dev-job-${prefix}-${activeExternalId}`;
    const dedupeKey = `${prefix}:${activeExternalId}`;
    const externalId = activeExternalId;
    const url =
      typeof (source.config as any)?.url === "string" && (source.config as any).url.startsWith("http")
        ? `${(source.config as any).url.replace(/\/$/, "")}/jobs/${activeExternalId}?gh_jid=${activeExternalId}`
        : `https://job-boards.greenhouse.io/${prefix}/jobs/${activeExternalId}?gh_jid=${activeExternalId}`;

    const isHourly = role.salary_min < 1000;
    const salaryRaw = isHourly
      ? `$${role.salary_min} - $${role.salary_max} / hr`
      : `$${role.salary_min.toLocaleString()} - $${role.salary_max.toLocaleString()}`;

    const activeWorkplaceType: WorkplaceType =
      (role as any).workplace_type ||
      (role.location.toLowerCase().includes("remote") ? "remote" : "onsite");

    const workplaceDesc =
      activeWorkplaceType === "onsite"
        ? `In-Office expectation (3-5 days/week in ${role.location})`
        : activeWorkplaceType === "remote"
        ? "Fully Remote"
        : `Hybrid in ${role.location}`;

    const description = `Position Overview:\nAs a ${role.title} at ${company}, you will work alongside our engineering teams to architect, deploy, and scale high-reliability systems and services. You will participate in architecture reviews, build telemetry monitoring, and write production-grade code.\n\nKey Responsibilities:\n- Collaborate with cross-functional teams to design, test, and ship resilient backend services.\n- Measure and optimize system performance, network latency, and availability.\n- Implement automated CI/CD pipelines and comprehensive integration testing.\n\nMinimum Qualifications:\n- Degree in Computer Science, Software Engineering, or equivalent practical experience.\n- Experience with one or more languages such as Rust, Go, Python, TypeScript, or C++.\n- Solid understanding of data structures, algorithms, and distributed systems fundamentals.\n\nCompensation & Benefits:\n- Competitive compensation (${salaryRaw}).\n- Workplace model: ${workplaceDesc}.\n- Mentorship from senior engineering staff and comprehensive learning stipends.`;

    return {
      id: jobId,
      dedupe_key: dedupeKey,
      source_id: source.id,
      external_id: externalId,
      title: role.title,
      company,
      company_icon_url: companyIconUrl,
      role_type: role.role_type,
      location: role.location,
      url,
      date_posted: now,
      date_added: now,
      last_seen_at: now,
      status: "scraped",
      status_changed_at: now,
      notes: null,
      favorited: false,
      favorited_at: null,
      salary_min: role.salary_min,
      salary_max: role.salary_max,
      salary_currency: "USD",
      salary_interval: isHourly ? "hour" : "year",
      salary_raw: salaryRaw,
      salary_source: "structured",
      workplace_type: activeWorkplaceType,
      sponsorship: "yes",
      sponsorship_evidence: "Sponsorship eligible for qualified candidates",
      clearance: "no",
      clearance_level: null,
      clearance_evidence: null,
      work_auth: "open",
      work_auth_evidence: "Sponsorship eligible for qualified candidates",
      experience_min: role.role_type === "internship" ? 0 : 2,
      experience_max: role.role_type === "internship" ? 1 : 5,
      experience_raw: role.role_type === "internship" ? "Currently enrolled student" : "2+ years",
      is_new_grad: role.role_type === "internship" ? "yes" : "no",
      experience_evidence: null,
      description_text: description,
      enrichment_version: 1,
      locations: [
        {
          raw: role.location,
          city: role.location.split(",")[0].trim(),
          region_code: role.location.includes(",") ? role.location.split(",")[1].trim() : null,
          country_code: role.location.includes("UK") ? "GB" : role.location.includes("Portugal") ? "PT" : "US",
          is_remote: activeWorkplaceType === "remote",
        },
      ],
    };
  });

  if (!source.filters) {
    return candidates;
  }

  const filters = source.filters as any;
  return candidates.filter((job) => {
    // 1. Role types
    if (Array.isArray(filters.role_types) && filters.role_types.length > 0) {
      if (!filters.role_types.includes(job.role_type)) {
        return false;
      }
    }

    const titleLower = job.title.toLowerCase();
    const locLower = (job.location || "").toLowerCase();

    // 2. Exclude keywords
    if (Array.isArray(filters.exclude_keywords) && filters.exclude_keywords.length > 0) {
      for (const kw of filters.exclude_keywords) {
        if (typeof kw === "string" && kw.trim().length > 0) {
          const kwLower = kw.trim().toLowerCase();
          if (titleLower.includes(kwLower) || locLower.includes(kwLower)) {
            return false;
          }
        }
      }
    }

    // 3. Include keywords
    if (Array.isArray(filters.include_keywords) && filters.include_keywords.length > 0) {
      const hasAny = filters.include_keywords.some((kw: string) => {
        if (typeof kw !== "string" || !kw.trim()) return false;
        const kwLower = kw.trim().toLowerCase();
        return titleLower.includes(kwLower) || locLower.includes(kwLower);
      });
      if (!hasAny) return false;
    }

    return true;
  });
}

export const api = {
  // Jobs
  listJobs: (filter?: ApiJobFilter): Promise<Page<Job>> => {
    if (!isTauri()) {
      let items = [...devJobs];
      if (filter) {
        if (filter.statuses && filter.statuses.length > 0) {
          items = items.filter((j) => filter.statuses!.includes(j.status));
        }
        if (filter.role_types && filter.role_types.length > 0) {
          items = items.filter((j) => filter.role_types!.includes(j.role_type));
        }
        if (filter.source_id) {
          items = items.filter((j) => j.source_id === filter.source_id);
        }
        if (filter.favorited_only) {
          items = items.filter((j) => j.favorited);
        }
        if (filter.search && filter.search.trim()) {
          const q = filter.search.toLowerCase().trim();
          items = items.filter(
            (j) =>
              j.title.toLowerCase().includes(q) ||
              j.company.toLowerCase().includes(q) ||
              (j.location && j.location.toLowerCase().includes(q)) ||
              (j.description_text && j.description_text.toLowerCase().includes(q))
          );
        }
      }
      const total = items.length;
      const offset = Number(filter?.offset || 0);
      const limit = Number(filter?.limit || 2000);
      const paged = items.slice(offset, offset + limit);
      return Promise.resolve({
        items: paged,
        total: BigInt(total),
        limit: BigInt(limit),
        offset: BigInt(offset),
      });
    }
    return invoke<Page<Job>>("list_jobs", { filter: filter || null });
  },

  getJob: (id: string): Promise<Job> => {
    if (!isTauri()) {
      const found = devJobs.find((j) => j.id === id);
      return found ? Promise.resolve({ ...found }) : Promise.reject(new Error(`Job ${id} not found`));
    }
    return invoke<Job>("get_job", { id });
  },

  updateJobStatus: (id: string, status: JobStatus): Promise<Job> => {
    if (!isTauri()) {
      const found = devJobs.find((j) => j.id === id);
      if (found) {
        found.status = status;
        found.status_changed_at = new Date().toISOString();
        saveDevJobs();
        return Promise.resolve({ ...found });
      }
      return Promise.reject(new Error(`Job ${id} not found`));
    }
    return invoke<Job>("update_job_status", { id, status });
  },

  bulkUpdateJobStatus: (ids: string[], status: JobStatus): Promise<number> => {
    if (!isTauri()) {
      let count = 0;
      const now = new Date().toISOString();
      for (const id of ids) {
        const found = devJobs.find((j) => j.id === id);
        if (found) {
          found.status = status;
          found.status_changed_at = now;
          count++;
        }
      }
      if (count > 0) saveDevJobs();
      return Promise.resolve(count);
    }
    return invoke<number>("bulk_update_job_status", { ids, status });
  },

  updateJobNotes: (id: string, notes?: string): Promise<Job> => {
    if (!isTauri()) {
      const found = devJobs.find((j) => j.id === id);
      if (found) {
        found.notes = notes || null;
        saveDevJobs();
        return Promise.resolve({ ...found });
      }
      return Promise.reject(new Error(`Job ${id} not found`));
    }
    return invoke<Job>("update_job_notes", { id, notes: notes || null });
  },

  getJobStatusEvents: (jobId: string) => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<JobStatusEvent[]>("get_job_status_events", { jobId });
  },

  toggleJobFavorite: (id: string): Promise<Job> => {
    if (!isTauri()) {
      const found = devJobs.find((j) => j.id === id);
      if (found) {
        found.favorited = !found.favorited;
        found.favorited_at = found.favorited ? new Date().toISOString() : null;
        saveDevJobs();
        return Promise.resolve({ ...found });
      }
      return Promise.reject(new Error(`Job ${id} not found`));
    }
    return invoke<Job>("toggle_job_favorite", { id });
  },

  setJobFavorite: (id: string, favorited: boolean): Promise<Job> => {
    if (!isTauri()) {
      const found = devJobs.find((j) => j.id === id);
      if (found) {
        found.favorited = favorited;
        found.favorited_at = favorited ? new Date().toISOString() : null;
        saveDevJobs();
        return Promise.resolve({ ...found });
      }
      return Promise.reject(new Error(`Job ${id} not found`));
    }
    return invoke<Job>("set_job_favorite", { id, favorited });
  },

  cleanupExpiredJobs: (sourceId?: string): Promise<number> => {
    if (!isTauri()) {
      const prevLength = devJobs.length;
      devJobs = devJobs.filter((j) => {
        if (sourceId && j.source_id !== sourceId) return true;
        if (j.favorited || j.status !== "scraped") return true;
        return false;
      });
      const pruned = prevLength - devJobs.length;
      if (pruned > 0) saveDevJobs();
      return Promise.resolve(pruned);
    }
    return invoke<number>("cleanup_expired_jobs", { sourceId: sourceId || null });
  },

  deleteJob: (id: string): Promise<boolean> => {
    if (!isTauri()) {
      const idx = devJobs.findIndex((j) => j.id === id);
      if (idx !== -1) {
        const deleted = devJobs[idx];
        devDeletedJobKeys.add(deleted.dedupe_key);
        saveDevDeletedJobKeys();
        devJobs.splice(idx, 1);
        saveDevJobs();
        return Promise.resolve(true);
      }
      return Promise.resolve(false);
    }
    return invoke<boolean>("delete_job", { id });
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
    if (!isTauri()) return Promise.resolve([...devSources]);
    return invoke<Source[]>("list_sources");
  },

  getSource: (id: string): Promise<Source> => {
    if (!isTauri()) {
      const found = devSources.find((s) => s.id === id);
      return found ? Promise.resolve(found) : Promise.reject(new Error(`Source ${id} not found`));
    }
    return invoke<Source>("get_source", { id });
  },

  seedSampleWorkflow: (): Promise<Source> => {
    if (!isTauri()) {
      const existing = devSources.find((s) => s.identifier === "cloudflare" && s.kind === "greenhouse");
      if (existing) return Promise.resolve(existing);
      const newSource = { ...SAMPLE_SWE_SOURCE, id: `sample-${Date.now()}` };
      devSources.push(newSource);
      return Promise.resolve(newSource);
    }
    return invoke<Source>("seed_sample_workflow");
  },

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
  }) => {
    if (!isTauri()) {
      const newSource: Source = {
        id: `source-${Date.now()}`,
        kind: input.kind as any,
        name: input.name,
        identifier: input.identifier,
        config: (input.config || {}) as any,
        enabled: input.enabled ?? true,
        last_run_at: null,
        last_status: null,
        last_error: null,
        created_at: new Date().toISOString(),
        cadence: input.cadence || "global",
        cadence_cron: input.cadence_cron || null,
        next_run_at: null,
        last_run_duration_ms: null,
        retention_days: BigInt(input.retention_days ?? 30),
        retention_policy: input.retention_policy || "untracked_only",
        filters: (input.filters || {}) as any,
      };
      devSources.push(newSource);
      return Promise.resolve(newSource);
    }
    return invoke<Source>("create_source", {
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
    });
  },

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
  }) => {
    if (!isTauri()) {
      const idx = devSources.findIndex((s) => s.id === input.id);
      if (idx >= 0) {
        devSources[idx] = {
          ...devSources[idx],
          ...(input.name !== undefined && input.name !== null ? { name: input.name } : {}),
          ...(input.identifier !== undefined && input.identifier !== null ? { identifier: input.identifier } : {}),
          ...(input.enabled !== undefined && input.enabled !== null ? { enabled: input.enabled } : {}),
          ...(input.cadence !== undefined && input.cadence !== null ? { cadence: input.cadence } : {}),
          ...(input.cadence_cron !== undefined ? { cadence_cron: input.cadence_cron } : {}),
          ...(input.retention_days !== undefined && input.retention_days !== null ? { retention_days: BigInt(input.retention_days) } : {}),
          ...(input.retention_policy !== undefined && input.retention_policy !== null ? { retention_policy: input.retention_policy } : {}),
        };
        return Promise.resolve(devSources[idx]);
      }
      return Promise.reject(new Error(`Source ${input.id} not found`));
    }
    return invoke<Source>("update_source", {
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
    });
  },

  deleteSource: (id: string) => {
    if (!isTauri()) {
      devSources = devSources.filter((s) => s.id !== id);
      return Promise.resolve();
    }
    return invoke<void>("delete_source", { id });
  },

  // Scraping & Scheduler
  runScrapeNow: async (): Promise<ScrapeRun> => {
    if (!isTauri()) {
      const enabled = devSources.filter((s) => s.enabled);
      const target = enabled[0] || devSources[0];
      if (target) {
        return api.runSourceNow(target.id);
      }
      return Promise.resolve({
        id: `mock-run-${Date.now()}`,
        trigger: "manual",
        status: "success",
        started_at: new Date().toISOString(),
        finished_at: new Date().toISOString(),
        jobs_found: BigInt(0),
        jobs_new: BigInt(0),
        error: null,
        source_id: null,
      });
    }
    return invoke<ScrapeRun>("run_scrape_now");
  },

  runSourceNow: async (id: string): Promise<ScrapeRun> => {
    if (!isTauri()) {
      const target = devSources.find((s) => s.id === id);
      const runId = `mock-run-${Date.now()}`;
      const startedAt = new Date().toISOString();
      const delayMs = executionDelayStep();

      const shouldFail =
        target?.identifier.includes("fail") ||
        target?.name.toLowerCase().includes("fail") ||
        target?.kind === ("unknown" as any);

      const runRecord: ScrapeRun = {
        id: runId,
        trigger: "single_source",
        status: "running" as any,
        started_at: startedAt,
        finished_at: null,
        jobs_found: BigInt(0),
        jobs_new: BigInt(0),
        error: null,
        source_id: id,
      };
      devSourceRuns.unshift(runRecord);

      const sourceName = target ? target.name : id;
      const sourceKind = target ? target.kind : "greenhouse";
      devLogs[runId] = [
        {
          id: `log-${Date.now()}-1`,
          run_id: runId,
          timestamp: new Date().toISOString(),
          level: "info",
          source_id: id,
          message: `Scrape workflow initiated: ${sourceName} (${sourceKind})`,
        },
      ];
      saveDevSourceRuns();
      saveDevLogs();

      await new Promise((r) => setTimeout(r, delayMs));

      if (shouldFail) {
        devLogs[runId].push({
          id: `log-${Date.now()}-2`,
          run_id: runId,
          timestamp: new Date().toISOString(),
          level: "warn",
          source_id: id,
          message: `Sidecar worker warning: Slow response from endpoint (>3000ms)`,
        });
        await new Promise((r) => setTimeout(r, delayMs));

        const finishedAt = new Date().toISOString();
        const errMsg = "Network timeout connecting to careers endpoint (HTTP 504 Gateway Timeout)";
        devLogs[runId].push({
          id: `log-${Date.now()}-3`,
          run_id: runId,
          timestamp: finishedAt,
          level: "error",
          source_id: id,
          message: `Failed to extract postings: ${errMsg}`,
        });
        devLogs[runId].push({
          id: `log-${Date.now()}-4`,
          run_id: runId,
          timestamp: finishedAt,
          level: "error",
          source_id: id,
          message: `Workflow run failed with exit code 1. 0 postings committed.`,
        });

        runRecord.status = "failed";
        runRecord.finished_at = finishedAt;
        runRecord.error = errMsg;

        if (target) {
          target.last_run_at = finishedAt;
          target.last_status = "error";
          target.last_error = errMsg;
          target.last_run_duration_ms = BigInt(delayMs * 2);
          saveDevSources();
        }

        saveDevSourceRuns();
        saveDevLogs();
        return runRecord;
      }

      devLogs[runId].push({
        id: `log-${Date.now()}-2`,
        run_id: runId,
        timestamp: new Date().toISOString(),
        level: "info",
        source_id: id,
        message: `Sidecar worker spawned. Fetching listings from configured endpoint...`,
      });
      saveDevLogs();

      await new Promise((r) => setTimeout(r, delayMs));

      const newJobs = target
        ? generateScrapedJobsForSource(target)
        : generateScrapedJobsForSource({ ...SAMPLE_SWE_SOURCE, id, name: "Target Workflow" });

      let newlyAddedCount = 0;
      let refreshedCount = 0;
      const nowIso = new Date().toISOString();

      for (const candidate of newJobs) {
        if (devDeletedJobKeys.has(candidate.dedupe_key)) {
          continue;
        }
        const existingIdx = devJobs.findIndex(
          (j) => j.dedupe_key === candidate.dedupe_key || j.url === candidate.url
        );
        if (existingIdx >= 0) {
          // Idempotent upsert: refresh last_seen_at and metadata, preserve user triage state!
          devJobs[existingIdx] = {
            ...devJobs[existingIdx],
            last_seen_at: nowIso,
            salary_min: candidate.salary_min ?? devJobs[existingIdx].salary_min,
            salary_max: candidate.salary_max ?? devJobs[existingIdx].salary_max,
            salary_raw: candidate.salary_raw ?? devJobs[existingIdx].salary_raw,
            company_icon_url: candidate.company_icon_url ?? devJobs[existingIdx].company_icon_url,
            description_text: candidate.description_text ?? devJobs[existingIdx].description_text,
          };
          refreshedCount++;
        } else {
          devJobs.unshift(candidate);
          newlyAddedCount++;
        }
      }
      saveDevJobs();

      const isCloudflare =
        target?.identifier === "cloudflare" ||
        target?.name?.toLowerCase().includes("cloudflare") ||
        id === SAMPLE_SWE_SOURCE.id;
      const extractedCount = isCloudflare ? 8 : newJobs.length;
      const matchedCount = newJobs.length;

      devLogs[runId].push({
        id: `log-${Date.now()}-3`,
        run_id: runId,
        timestamp: new Date().toISOString(),
        level: "info",
        source_id: id,
        message: `Extracted ${extractedCount} postings from source. Policy engine matched ${matchedCount} postings against criteria.`,
      });

      const finishedAt = new Date().toISOString();
      const durationMs = delayMs * 2;

      devLogs[runId].push({
        id: `log-${Date.now()}-4`,
        run_id: runId,
        timestamp: finishedAt,
        level: "info",
        source_id: id,
        message:
          newlyAddedCount > 0
            ? `Workflow run completed in ${(durationMs / 1000).toFixed(2)}s with status: SUCCESS. Committed ${newlyAddedCount} new postings to Discovered inbox (${refreshedCount} existing postings refreshed).`
            : `Workflow run completed in ${(durationMs / 1000).toFixed(2)}s with status: SUCCESS. 0 new postings found (${refreshedCount} existing postings refreshed). Discovered inbox preserved without duplicates.`,
      });

      runRecord.status = "success";
      runRecord.finished_at = finishedAt;
      runRecord.jobs_found = BigInt(extractedCount);
      runRecord.jobs_new = BigInt(newlyAddedCount);

      if (target) {
        target.last_run_at = finishedAt;
        target.last_status = "ok";
        target.last_error = null;
        target.last_run_duration_ms = BigInt(durationMs);
        saveDevSources();
      }

      saveDevSourceRuns();
      saveDevLogs();
      return runRecord;
    }
    return invoke<ScrapeRun>("run_source_now", { id });
  },

  listScrapeRuns: (limit?: number): Promise<ScrapeRun[]> => {
    if (!isTauri()) return Promise.resolve(devSourceRuns.slice(0, limit ?? 50));
    return invoke<ScrapeRun[]>("list_scrape_runs", { limit });
  },

  listSourceRuns: (sourceId: string, limit?: number): Promise<ScrapeRun[]> => {
    if (!isTauri()) {
      const runs = devSourceRuns.filter((r) => r.source_id === sourceId);
      return Promise.resolve(runs.slice(0, limit ?? 10));
    }
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

  testSource: (id: string): Promise<WorkerEvent[]> => {
    if (!isTauri()) {
      const source = devSources.find((s) => s.id === id);
      const jobs = source ? generateScrapedJobsForSource(source) : [];
      const events: WorkerEvent[] = jobs.map((j) => ({
        type: "job",
        source_id: id,
        external_id: j.external_id || "ext-1",
        title: j.title,
        company: j.company,
        role_type: j.role_type,
        location: j.location || "Remote",
        url: j.url,
        date_posted: j.date_posted,
        salary: j.salary_min
          ? {
              min: j.salary_min,
              max: j.salary_max,
              currency: j.salary_currency || "USD",
              interval: j.salary_interval || "hour",
              raw: j.salary_raw || "",
              source: "structured",
            }
          : null,
        description_text: j.description_text,
        company_icon_url: j.company_icon_url || null,
        locations: j.locations,
        workplace_type: j.workplace_type,
        sponsorship: null,
        clearance: null,
        work_auth: null,
        experience: null,
        is_new_grad: j.is_new_grad,
      }));
      return Promise.resolve(events);
    }
    return invoke<WorkerEvent[]>("test_source", { id });
  },

  testSourceTransient: (input: TransientSourceInput): Promise<WorkerEvent[]> => {
    if (!isTauri()) {
      const dummySource: Source = {
        ...SAMPLE_SWE_SOURCE,
        name: input.name,
        identifier: input.identifier,
        kind: input.kind as any,
        config: input.config as any,
      };
      const jobs = generateScrapedJobsForSource(dummySource);
      const events: WorkerEvent[] = jobs.map((j) => ({
        type: "job",
        source_id: "transient",
        external_id: j.external_id || "ext-1",
        title: j.title,
        company: j.company,
        role_type: j.role_type,
        location: j.location || "Remote",
        url: j.url,
        date_posted: j.date_posted,
        salary: j.salary_min
          ? {
              min: j.salary_min,
              max: j.salary_max,
              currency: j.salary_currency || "USD",
              interval: j.salary_interval || "hour",
              raw: j.salary_raw || "",
              source: "structured",
            }
          : null,
        description_text: j.description_text,
        company_icon_url: j.company_icon_url || null,
        locations: j.locations,
        workplace_type: j.workplace_type,
        sponsorship: null,
        clearance: null,
        work_auth: null,
        experience: null,
        is_new_grad: j.is_new_grad,
      }));
      return Promise.resolve(events);
    }
    return invoke<WorkerEvent[]>("test_source_transient", { input });
  },

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
    if (!isTauri()) {
      const found = devSourceRuns.find((r) => r.id === runId);
      return found ? Promise.resolve({ ...found }) : Promise.reject(new Error(`Run ${runId} not found`));
    }
    return invoke<ScrapeRun>("get_scrape_run", { runId });
  },

  getScrapeLogs: (runId: string): Promise<ScrapeLogEntry[]> => {
    if (!isTauri()) {
      return Promise.resolve(devLogs[runId] ? [...devLogs[runId]] : []);
    }
    return invoke<ScrapeLogEntry[]>("get_scrape_logs", { runId });
  },

  getMetrics: (granularity: "week" | "month" = "week", periods: number = 12): Promise<Metrics> => {
    if (!isTauri()) {
      const scraped = devJobs.filter((j) => j.status === "scraped" || j.status === "viewed").length;
      const applied = devJobs.filter((j) => j.status === "applied").length;
      const interviewing = devJobs.filter((j) => j.status === "interviewing").length;
      const offers = devJobs.filter((j) => j.status === "offer").length;
      const favorited = devJobs.filter((j) => j.favorited).length;
      const totalTracked = applied + interviewing + offers;
      const responseRate = totalTracked > 0 ? ((interviewing + offers) / totalTracked) * 100 : 0;
      const salaries = devJobs.filter((j) => j.salary_min != null).map((j) => j.salary_min!);
      const median_salary = salaries.length > 0 ? 155000 : null;
      const avg_salary = salaries.length > 0 ? 160000 : null;

      return Promise.resolve({
        totals: {
          scraped,
          applied,
          interviewing,
          offers,
          favorited,
          response_rate: responseRate,
          median_salary,
          avg_salary,
        },
        series: [],
        by_role_type: [],
        salary_distribution: [],
        streak: { current: applied > 0 ? 1 : 0, longest: applied > 0 ? 1 : 0, last_applied_on: null },
      });
    }
    return invoke<Metrics>("get_metrics", { granularity, periods });
  },

  createNotification: (input: CreateNotificationInput): Promise<AppNotification> => {
    if (!isTauri()) {
      const newNotif: AppNotification = {
        id: `notif-${Date.now()}`,
        kind: input.kind,
        title: input.title,
        message: input.message,
        action_label: input.action_label || null,
        action_payload: input.action_payload || null,
        read: false,
        created_at: new Date().toISOString(),
      };
      devNotifications.unshift(newNotif);
      return Promise.resolve(newNotif);
    }
    return invoke<AppNotification>("create_notification", { input });
  },

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

  openTerminalConsole: (): Promise<void> => {
    if (!isTauri()) {
      console.log("[Dev] openTerminalConsole invoked");
      return Promise.resolve();
    }
    return invoke<void>("open_terminal_console");
  },
};
