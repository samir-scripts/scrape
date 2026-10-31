import type { HttpProxyStatus } from "../bindings/HttpProxyStatus";
import type { JsonValue } from "../bindings/serde_json/JsonValue";
import type { RoleTitleSuggestion } from "../bindings/RoleTitleSuggestion";
import type { CompanySuggestion } from "../bindings/CompanySuggestion";
import type { FilterCompatibilityRestriction } from "../bindings/FilterCompatibilityRestriction";
import type { WorkflowFilters } from "../bindings/WorkflowFilters";
import type { WorkflowReplacementPreview } from "../bindings/WorkflowReplacementPreview";
import type { WorkflowReplacementSelection } from "../bindings/WorkflowReplacementSelection";
import type { WorkflowReplacementReport } from "../bindings/WorkflowReplacementReport";
import { INGESTION_LIMITS } from "../bindings/ingestionLimits";
import { errorMessage } from "./errors";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import type { CompanyLogo } from "../bindings/CompanyLogo";
import { workflowSpecToCreateInput, workflowSpecToYaml, validateWorkflowYaml, SAMPLE_WORKFLOW_TEMPLATES } from "./yamlWorkflow";
import { matchesJob } from "./filter";
import type { RemovedPosting } from "../bindings/RemovedPosting";
import type { RemovalResult } from "../bindings/RemovalResult";
import { needsDesktop, isFixtureMode, isRecordedMode } from "./runtime";
import { recordedJobs } from "./recordedJobs";
const storagePrefix = isFixtureMode() ? "scrape:demo:v1" : "scrape:recorded:v1";
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
import type { RetentionStatus } from "../bindings/RetentionStatus";
import type { BrowserRuntimeStatus } from "../bindings/BrowserRuntimeStatus";
import type { BrowserSessionSummary } from "../bindings/BrowserSessionSummary";
import type { WorkflowExtractor } from "../bindings/WorkflowExtractor";
import type { ExtractorTestResult } from "../bindings/ExtractorTestResult";
import type { ScriptEnvironmentStatus } from "../bindings/ScriptEnvironmentStatus";
import type { WorkflowScriptConfig } from "../bindings/WorkflowScriptConfig";

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
  scraper_respect_robots: false,
  scraper_per_host_delay_ms: 750,
  scraper_jitter_ms: 500,
  scraper_max_jobs_per_run: 20,
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
      const saved = localStorage.getItem(`${storagePrefix}:settings`);
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

function demoSourceFromTemplate(templateId: string, id: string): Source {
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(candidate => candidate.id === templateId);
  if (!template) throw new Error(`Missing demonstration workflow ${templateId}`);
  const input = workflowSpecToCreateInput(template.spec);
  const config: Record<string, JsonValue> = input.config && typeof input.config === "object" && !Array.isArray(input.config)
    ? { ...input.config as Record<string, JsonValue> }
    : {};
  config.__builtin_workflow = { catalog_version: 3, template_id: template.id, fingerprint: JSON.stringify(template.spec) };
  return {
    id,
    kind: input.kind as Source["kind"],
    name: template.name,
    identifier: input.identifier,
    config,
    enabled: true,
    last_run_at: null,
    last_status: null,
    last_error: null,
    created_at: new Date().toISOString(),
    cadence: "manual",
    cadence_cron: null,
    next_run_at: null,
    last_run_duration_ms: null,
    retention_days: BigInt(input.retention_days ?? 30),
    retention_policy: input.retention_policy ?? "untracked_only",
    filters: input.filters ?? {},
    needs_review: false,
    script_trusted: false,
  };
}

export const SAMPLE_ENGINEERING_SOURCE = demoSourceFromTemplate("software-engineering-last-12-hours", "demo-software-engineering");
export const SAMPLE_FAST_FOOD_SOURCE = demoSourceFromTemplate("cherry-hill-fast-food-last-month", "demo-fast-food");
export const SAMPLE_REMOTE_SOURCE = demoSourceFromTemplate("remote-usd-over-60000", "demo-remote-salary");
const BUILTIN_DEMO_SOURCES = [SAMPLE_ENGINEERING_SOURCE, SAMPLE_FAST_FOOD_SOURCE, SAMPLE_REMOTE_SOURCE];
let devSources: Source[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`${storagePrefix}:sources`);
      if (saved) return JSON.parse(saved, (key, value) => ["retention_days", "duration_ms"].includes(key) && typeof value === "number" ? BigInt(value) : value);
    }
  } catch {}
  return isFixtureMode() ? BUILTIN_DEMO_SOURCES.map(source => ({ ...source, config: structuredClone(source.config), filters: structuredClone(source.filters) })) : [];
})();

function saveDevSources() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(`${storagePrefix}:sources`, JSON.stringify(devSources, (_, value) => typeof value === "bigint" ? Number(value) : value));
    }
  } catch {}
}

const LEGACY_DEMO_SAMPLE_IDS = new Set(["sample-swe-internships-1"]);
function demoWorkflowMarker(source: Source): { template_id?: string } {
  const config = source.config && typeof source.config === "object" && !Array.isArray(source.config)
    ? source.config as Record<string, unknown>
    : {};
  const marker = config.__builtin_workflow;
  return marker && typeof marker === "object" ? marker as { template_id?: string } : {};
}
function demoReplacementFingerprint(source: Source): string {
  return JSON.stringify(source, (_, value) => typeof value === "bigint" ? Number(value) : value);
}
function exactDemoBuiltin(source: Source, templateId: string): boolean {
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(candidate => candidate.id === templateId);
  const config = source.config && typeof source.config === "object" && !Array.isArray(source.config)
    ? source.config as Record<string, unknown>
    : {};
  return !!template && demoWorkflowMarker(source).template_id === templateId &&
    JSON.stringify(config.__workflow_spec) === JSON.stringify(template.spec);
}

let devJobs: Job[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`${storagePrefix}:jobs`);
      if (saved) return JSON.parse(saved, (key, value) => ["retention_days", "duration_ms"].includes(key) && typeof value === "number" ? BigInt(value) : value);
    }
  } catch {}
  return isFixtureMode() ? [] : recordedJobs();
})();

function saveDevJobs() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(`${storagePrefix}:jobs`, JSON.stringify(devJobs));
    }
  } catch {}
}

let devSourceRuns: ScrapeRun[] = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`${storagePrefix}:runs`);
      if (saved) return JSON.parse(saved, (key, value) => ["retention_days", "duration_ms"].includes(key) && typeof value === "number" ? BigInt(value) : value);
    }
  } catch {}
  return [];
})();

function saveDevSourceRuns() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(`${storagePrefix}:runs`, JSON.stringify(devSourceRuns, (_, value) => typeof value === "bigint" ? Number(value) : value));
    }
  } catch {}
}

let devLogs: Record<string, ScrapeLogEntry[]> = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`${storagePrefix}:logs`);
      if (saved) return JSON.parse(saved, (key, value) => ["retention_days", "duration_ms"].includes(key) && typeof value === "number" ? BigInt(value) : value);
    }
  } catch {}
  return {};
})();

function saveDevLogs() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(`${storagePrefix}:logs`, JSON.stringify(devLogs));
    }
  } catch {}
}

let devDeletedJobKeys: Set<string> = (() => {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`${storagePrefix}:deleted_jobs`);
      if (saved) return new Set(JSON.parse(saved));
    }
  } catch {}
  return new Set();
})();

function saveDevDeletedJobKeys() {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(`${storagePrefix}:deleted_jobs`, JSON.stringify([...devDeletedJobKeys]));
    }
  } catch {}
}

let demoBlocks: RemovedPosting[] = (() => {
  try { return JSON.parse(localStorage.getItem(`${storagePrefix}:blocks`) || "[]"); } catch { return []; }
})();
function saveDemoBlocks() {
  try { localStorage.setItem(`${storagePrefix}:blocks`, JSON.stringify(demoBlocks)); } catch {}
}
const demoIdentity = (job: Job) => job.posting_identity || job.dedupe_key;
function removeDemoJob(id: string, block: boolean): RemovalResult {
  const job = devJobs.find((j) => j.id === id);
  if (!job) throw new Error("Posting not found");
  const identity = demoIdentity(job);
  const date = new Date().toISOString();
  let copies = 0;
  devJobs.forEach((j) => { if (demoIdentity(j) === identity && !j.removed_at) { j.removed_at = date; copies++; } });
  if (block && !demoBlocks.some((b) => b.identity === identity)) {
    demoBlocks.push({ identity, title: job.title, company: job.company, url: job.url,
      source_name: devSources.find((s) => s.id === job.source_id)?.name || null,
      removed_at: date, blocked: true, blocked_at: date, copies });
  }
  saveDevJobs(); saveDemoBlocks();
  return { identity, copies };
}

const executionDelayStep = () => {
  if (typeof process !== "undefined" && process.env?.NODE_ENV === "test") {
    return 15;
  }
  return 500;
};

export function __resetDevData() {
  devSources = BUILTIN_DEMO_SOURCES.map(source => ({ ...source, config: structuredClone(source.config), filters: structuredClone(source.filters) }));
  devJobs = [];
  devSourceRuns = [];
  devLogs = {};
  devDeletedJobKeys.clear();
  demoBlocks = [];
  saveDemoBlocks();
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

export function generateScrapedJobsForSource(source: Source, applyFilters = true): Job[] {
  if (!isFixtureMode()) throw new Error("Synthetic fixtures are restricted to test harnesses.");
  const now = new Date().toISOString();
  const isCloudflare = source.identifier.toLowerCase() === "cloudflare";
  const company = "Demo Company";

  const rawSampleRoles = isCloudflare
    ? [
        {
          external_id: "8199958",
          title: "Software Engineer Intern (2027) - Austin, TX",
          location: "Austin, TX",
          salary_min: 50,
          salary_max: 60,
          salary_interval: "hour" as const,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8245197",
          title: "Software Engineer Intern (2027)",
          location: "Lisbon, Portugal",
          salary_min: 50,
          salary_max: 60,
          salary_interval: "hour" as const,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8245211",
          title: "Software Engineer Intern (2027)",
          location: "London, UK",
          salary_min: 52,
          salary_max: 62,
          salary_interval: "hour" as const,
          role_type: "internship" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "8187404",
          title: "Systems Engineer, Network Protocols & Distributed Systems",
          location: "Austin, TX",
          salary_min: 55,
          salary_max: 65,
          salary_interval: "hour" as const,
          role_type: "internship" as const,
          workplace_type: "hybrid" as const,
        },
        // Fictional roles used to exercise policy exclusion:
        {
          external_id: "8247492",
          title: "Technical Product Marketing Manager",
          location: "Remote",
          salary_min: 140000,
          salary_max: 180000,
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "remote" as const,
        },
        {
          external_id: "7695702",
          title: "Account Executive, FedCiv",
          location: "Washington DC, US",
          salary_min: 130000,
          salary_max: 170000,
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "8097321",
          title: "AI Security Research & Red Team Engineer",
          location: "San Francisco, CA",
          salary_min: 165000,
          salary_max: 215000,
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "7742773",
          title: "Systems Engineer, MAPS",
          location: "Austin, TX",
          salary_min: 150000,
          salary_max: 190000,
          salary_interval: "year" as const,
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
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "hybrid" as const,
        },
        {
          external_id: "102",
          title: `Infrastructure Engineer - Platform & Reliability`,
          location: "Remote",
          salary_min: 155000,
          salary_max: 195000,
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "remote" as const,
        },
        {
          external_id: "103",
          title: `Full Stack Engineer - Product Experience`,
          location: "New York, NY",
          salary_min: 140000,
          salary_max: 175000,
          salary_interval: "year" as const,
          role_type: "full_time" as const,
          workplace_type: "onsite" as const,
        },
        {
          external_id: "104",
          title: `Systems Engineer Intern - Distributed Data`,
          location: "Remote",
          salary_min: 45,
          salary_max: 55,
          salary_interval: "hour" as const,
          role_type: "internship" as const,
          workplace_type: "remote" as const,
        },
      ];

  const prefix = source.identifier || "src";
  const companyIconUrl = null;

  const candidates: Job[] = rawSampleRoles.map((role, idx) => {
    const activeExternalId = `demo-${idx + 1}`;
    const jobId = `dev-job-${prefix}-${activeExternalId}`;
    const dedupeKey = `${prefix}:${activeExternalId}`;
    const externalId = activeExternalId;
    const url = `https://example.com/demo/${source.kind}/${encodeURIComponent(prefix)}/jobs/${externalId}`;
    const isHourly = role.salary_interval === "hour";
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

    const description = `FICTIONAL DEMONSTRATION POSTING ${externalId}\n\nPosition Overview:\nAs a ${role.title} at ${company}, you will work alongside our engineering teams to architect, deploy, and scale high-reliability systems and services. You will participate in architecture reviews, build telemetry monitoring, and write production-grade code.\n\nKey Responsibilities:\n- Collaborate with cross-functional teams to design, test, and ship resilient backend services.\n- Measure and optimize system performance, network latency, and availability.\n- Implement automated CI/CD pipelines and comprehensive integration testing.\n\nMinimum Qualifications:\n- Degree in Computer Science, Software Engineering, or equivalent practical experience.\n- Experience with one or more languages such as Rust, Go, Python, TypeScript, or C++.\n- Solid understanding of data structures, algorithms, and distributed systems fundamentals.\n\nCompensation & Benefits:\n- Competitive compensation (${salaryRaw}).\n- Workplace model: ${workplaceDesc}.\n- Mentorship from senior engineering staff and comprehensive learning stipends.`;

    return {
      id: jobId,
      dedupe_key: dedupeKey,
      posting_identity: `demo:${prefix}:${externalId}`,
      removed_at: null,
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
      compensation: [{ min_minor: Math.round(role.salary_min * 100), max_minor: Math.round(role.salary_max * 100), currency: "USD", interval: role.salary_interval, raw: salaryRaw, source: "structured", derived: false, location: role.location }],
      salary_min: isHourly ? role.salary_min * 2080 : role.salary_min,
      salary_max: isHourly ? role.salary_max * 2080 : role.salary_max,
      salary_annual_min: isHourly ? role.salary_min * 2080 : role.salary_min,
      salary_annual_max: isHourly ? role.salary_max * 2080 : role.salary_max,
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

  return applyFilters ? candidates.filter(job => matchesDemoWorkflow(job, source)) : candidates;
}

// Demonstration semantics are deliberately limited and never used by desktop execution.
function matchesDemoWorkflow(job: Job, source: Source): boolean {
  if (source.filters && (typeof source.filters !== "object" || Array.isArray(source.filters))) throw new Error("Workflow filters must be an object.");
  const filters = source.filters as WorkflowSpec["filters"] || {};
  if (filters.query || filters.policy) throw new Error("Expert queries and policies require desktop validation.");
  const rules = filters.matching;
  const title = job.title.toLocaleLowerCase("en-US");
  if (rules?.target_roles?.length && !rules.target_roles.some(role => title.includes(role.toLocaleLowerCase("en-US")))) return false;
  if (rules?.companies?.length && !rules.companies.some(company => job.company.toLocaleLowerCase("en-US").includes(company.toLocaleLowerCase("en-US")))) return false;
  if (rules?.experience_match_mode === "required_minimum" && (rules.experience_min != null || rules.experience_max != null)) throw new Error("Minimum required experience filtering requires the desktop application.");
  const dateFilter = rules?.posting_date;
  if (dateFilter) {
    if (dateFilter.preset && !["past_12h", "past_24h", "past_48h", "past_week", "past_month", "any"].includes(dateFilter.preset)) {
      throw new Error("This publication-date rule requires the desktop application.");
    }
    if (!job.date_posted) {
      if (!dateFilter.include_unknown) return false;
    } else {
      const posted = Date.parse(job.date_posted);
      if (!Number.isFinite(posted)) {
        if (!dateFilter.include_unknown) return false;
      } else {
        const now = Date.now();
        const hours = dateFilter.preset === "any" ? null : dateFilter.last_hours ?? ({
          past_12h: 12, past_24h: 24, past_48h: 48, past_week: 168, past_month: 720,
        } as Record<string, number>)[dateFilter.preset || ""] ?? null;
        if (hours != null && (posted < now - hours * 3_600_000 || posted > now)) return false;
        if (dateFilter.from && posted < Date.parse(`${dateFilter.from}T00:00:00Z`)) return false;
        if (dateFilter.through && posted > Date.parse(`${dateFilter.through}T23:59:59.999Z`)) return false;
      }
    }
  }
  const includes = filters.include_keywords;
  const excludes = filters.exclude_keywords;
  const locations = filters.locations;
  const roles = filters.role_types;
  const text = `${job.title} ${job.location || ""}`.toLocaleLowerCase("en-US");
  const contains = (word: string) => text.includes(word.toLocaleLowerCase("en-US"));
  if (includes && includes.length && !includes.some(contains)) return false;
  if (excludes && excludes.length && excludes.some(contains)) return false;
  if (locations && locations.length && !locations.some(loc => (job.location || "").toLocaleLowerCase("en-US").includes(loc.toLocaleLowerCase("en-US")))) return false;
  if (roles && roles.length && !roles.includes(job.role_type)) return false;
  const keywords = rules?.keywords;
  if (keywords) {
    const values = { title: job.title, company: job.company, location: job.location || "", description: job.description_text || "" };
    const haystack = keywords.fields.map(field => values[field as keyof typeof values] || "").join(" ").toLocaleLowerCase("en-US");
    const match = (word: string) => haystack.includes(word.toLocaleLowerCase("en-US"));
    if (keywords.any?.length && !keywords.any.some(match)) return false;
    if (keywords.all?.length && !keywords.all.every(match)) return false;
    if (keywords.exclude?.length && keywords.exclude.some(match)) return false;
  }
  if (rules?.workplaces?.length && !rules.workplaces.includes(job.workplace_type)) return false;
  const pay = rules?.compensation || filters.compensation;
  if (pay?.match_mode === "advertised_amount") {
    const accepted = (job.compensation || []).some(range => {
      if (range.derived || range.currency?.toUpperCase() !== pay.currency?.toUpperCase() || range.interval !== pay.interval) return false;
      const upper = range.max_minor ?? range.min_minor;
      const lower = range.min_minor ?? range.max_minor;
      return upper != null && lower != null &&
        (pay.min_minor == null || upper >= pay.min_minor) &&
        (pay.max_minor == null || lower <= pay.max_minor);
    });
    if (!accepted) return false;
  }
  return matchesJob(job, {
    role_types: rules?.role_types, experience_min: rules?.experience_min, experience_max: rules?.experience_max,
    experience_include_unknown: rules?.experience_include_unknown, new_grad: rules?.new_grad,
    sponsorship: rules?.sponsorship, clearance: rules?.clearance, work_auth: rules?.work_auth,
    countries: rules?.countries, regions: rules?.regions, cities: rules?.cities, location_include_unknown: rules?.location_include_unknown,
    salary_min: pay?.match_mode === "advertised_amount" || pay?.min_minor == null ? undefined : pay.min_minor / 100,
    salary_max: pay?.match_mode === "advertised_amount" || pay?.max_minor == null ? undefined : pay.max_minor / 100,
    salary_interval: pay?.match_mode === "advertised_amount" ? undefined : pay?.interval,
    salary_currency: pay?.match_mode === "advertised_amount" ? undefined : pay?.currency,
    salary_include_unknown: pay?.include_unknown,
  });
}

const apiImplementation = {
  previewDataCleanup: (mode: import("../bindings/CleanupMode").CleanupMode): Promise<import("../bindings/CleanupPreview").CleanupPreview> => {
    if (!isTauri()) return Promise.reject(new Error("Data cleanup is available in the desktop app only. Browser previews cannot clear the desktop database."));
    return invoke("preview_data_cleanup", { mode });
  },
  executeDataCleanup: (request: import("../bindings/CleanupRequest").CleanupRequest): Promise<import("../bindings/CleanupResult").CleanupResult> => {
    if (!isTauri()) return Promise.reject(new Error("Data cleanup is available in the desktop app only. Browser previews cannot clear the desktop database."));
    return invoke("execute_data_cleanup", { request });
  },

  // Jobs
  listJobs: (filter?: ApiJobFilter): Promise<Page<Job>> => {
    if (!isTauri()) {
      for (const rules of [filter, filter?.matching]) {
        if (rules?.experience_match_mode === "required_minimum" && (rules.experience_min != null || rules.experience_max != null)) return Promise.reject(new Error("Minimum required experience filtering requires the desktop application."));
      }
      if (filter?.matching?.target_roles?.length || filter?.matching?.companies?.length) return Promise.reject(new Error("Role and company filtering requires the desktop application."));
      let items = devJobs.filter((job) => !job.removed_at && matchesJob(job, filter || {}));
      const sort = filter?.sort || "date_added_desc";
      const compare = (a: Job, b: Job) => {
        switch (sort) {
          case "company_desc": return b.company.localeCompare(a.company);
          case "title_desc": return b.title.localeCompare(a.title);
          case "status_asc": return a.status.localeCompare(b.status);
          case "status_desc": return b.status.localeCompare(a.status);
          case "favorited_asc": return Number(a.favorited) - Number(b.favorited);
          case "company_asc": return a.company.localeCompare(b.company);
          case "title_asc": return a.title.localeCompare(b.title);
          case "date_added_asc": return a.date_added.localeCompare(b.date_added);
          case "date_posted_asc": return (a.date_posted || "").localeCompare(b.date_posted || "");
          case "date_posted_desc": return (b.date_posted || "").localeCompare(a.date_posted || "");
          case "salary_asc": return (a.salary_annual_min ?? a.salary_min ?? -1) - (b.salary_annual_min ?? b.salary_min ?? -1);
          case "salary_desc": return (b.salary_annual_max ?? b.salary_max ?? -1) - (a.salary_annual_max ?? a.salary_max ?? -1);
          case "favorited_desc": return Number(b.favorited) - Number(a.favorited);
          case "experience_asc": return (a.experience_min ?? -1) - (b.experience_min ?? -1);
          case "experience_desc": return (b.experience_min ?? -1) - (a.experience_min ?? -1);
          default: return b.date_added.localeCompare(a.date_added);
        }
      };
      items = items.sort((a, b) => compare(a, b) || a.id.localeCompare(b.id));
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

  getCompanyLogoAsset: (assetReference: string): Promise<string> => invoke<string>("get_company_logo_asset", { assetReference }),

  ensureCompanyLogoForJob: (jobId: string): Promise<CompanyLogo | null> => {
    if (!isTauri()) {
      return Promise.resolve(null);
    }
    return invoke<CompanyLogo | null>("ensure_company_logo_for_job", { jobId });
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
        if (j.removed_at || j.notes?.trim() || j.favorited || j.status !== "scraped") return true;
        return false;
      });
      const pruned = prevLength - devJobs.length;
      if (pruned > 0) saveDevJobs();
      return Promise.resolve(pruned);
    }
    return invoke<number>("cleanup_expired_jobs", { sourceId: sourceId || null });
  },

  getRetentionStatus: (): Promise<RetentionStatus | null> => {
    if (!isTauri()) return Promise.resolve(null);
    return invoke<RetentionStatus>("get_retention_status");
  },

  deleteJob: (id: string): Promise<boolean> => {
    if (!isTauri()) {
      if (!devJobs.some((j) => j.id === id)) return Promise.resolve(false);
      return Promise.resolve(removeDemoJob(id, false).copies > 0);
    }
    return invoke<boolean>("delete_job", { id });
  },
  removeJob: (id: string, block: boolean): Promise<RemovalResult> => {
    if (!isTauri()) return Promise.resolve(removeDemoJob(id, block));
    return invoke<RemovalResult>("remove_job", { id, block });
  },
  postingSavedCopies: (id: string): Promise<number> => {
    if (!isTauri()) {
      const job = devJobs.find((j) => j.id === id);
      return Promise.resolve(job ? devJobs.filter((j) => !j.removed_at && demoIdentity(j) === demoIdentity(job)).length : 0);
    }
    return invoke<number>("posting_saved_copies", { id });
  },
  restorePosting: (identity: string): Promise<void> => {
    if (!isTauri()) {
      devJobs.forEach((j) => { if (demoIdentity(j) === identity) j.removed_at = null; });
      demoBlocks = demoBlocks.filter((b) => b.identity !== identity);
      devDeletedJobKeys.delete(identity);
      saveDevJobs(); saveDemoBlocks(); saveDevDeletedJobKeys();
      return Promise.resolve();
    }
    return invoke<void>("restore_posting", { identity });
  },
  setPostingBlocked: (identity: string, blocked: boolean): Promise<void> => {
    if (!isTauri()) {
      const job = devJobs.find((j) => demoIdentity(j) === identity);
      if (blocked && job && !demoBlocks.some((b) => b.identity === identity)) {
        demoBlocks.push({ identity, title: job.title, company: job.company, url: job.url,
          source_name: devSources.find((s) => s.id === job.source_id)?.name || null,
          removed_at: job.removed_at || null, copies: 1, blocked: true, blocked_at: new Date().toISOString() });
      }
      if (!blocked) { demoBlocks = demoBlocks.filter((b) => b.identity !== identity); devDeletedJobKeys.delete(identity); }
      saveDemoBlocks(); saveDevDeletedJobKeys();
      return Promise.resolve();
    }
    return invoke<void>("set_posting_blocked", { identity, blocked });
  },
  listRemovedPostings: (search = "", view = "all", offset = 0, limit = 25): Promise<Page<RemovedPosting>> => {
    if (!isTauri()) {
      const grouped = new Map<string, RemovedPosting>();
      for (const job of devJobs.filter((j) => j.removed_at)) {
        const identity = demoIdentity(job), block = demoBlocks.find((b) => b.identity === identity);
        const entry = grouped.get(identity);
        if (entry) { entry.copies++; continue; }
        grouped.set(identity, { identity, title: job.title, company: job.company, url: job.url,
          source_name: devSources.find((s) => s.id === job.source_id)?.name || null,
          removed_at: job.removed_at || null, blocked: !!block, blocked_at: block?.blocked_at || null, copies: 1 });
      }
      demoBlocks.forEach((block) => { if (!grouped.has(block.identity)) grouped.set(block.identity, block); });
      const items = [...grouped.values()].filter((p) => (view === "all" || (view === "blocked" ? p.blocked : !!p.removed_at)) &&
        `${p.title} ${p.company} ${p.source_name || ""}`.toLowerCase().includes(search.toLowerCase()));
      return Promise.resolve({ items: items.slice(offset, offset + limit), total: BigInt(items.length), offset: BigInt(offset), limit: BigInt(limit) });
    }
    return invoke<Page<RemovedPosting>>("list_removed_postings", { search, view, offset, limit });
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
          localStorage.setItem(`${storagePrefix}:settings`, JSON.stringify(devSettings));
        }
      } catch {}
      return Promise.resolve({ ...devSettings });
    }
    return invoke<Settings>("update_setting", { key, value });
  },

  getHttpProxyStatus: (): Promise<HttpProxyStatus> => {
    if (!isTauri()) return Promise.resolve({ configured: false, server: null, authenticated: false });
    return invoke<HttpProxyStatus>("get_http_proxy_status");
  },
  setHttpProxy: (server: string, username?: string | null, password?: string | null): Promise<HttpProxyStatus> => {
    if (!isTauri()) return Promise.reject(new Error("Proxy configuration is available only in the desktop app."));
    return invoke<HttpProxyStatus>("set_http_proxy", { server, username: username || null, password: password || null });
  },
  clearHttpProxy: (): Promise<HttpProxyStatus> => {
    if (!isTauri()) return Promise.reject(new Error("Proxy configuration is available only in the desktop app."));
    return invoke<HttpProxyStatus>("clear_http_proxy");
  },

  getAdzunaCredentialStatus: (): Promise<{ configured: boolean }> => {
    if (!isTauri()) return Promise.resolve({ configured: false });
    return invoke<{ configured: boolean }>("get_adzuna_credential_status");
  },
  setAdzunaCredentials: (appId: string, appKey: string): Promise<{ configured: boolean }> => {
    if (!isTauri()) return Promise.reject(new Error("Adzuna credentials are available only in the desktop app."));
    return invoke<{ configured: boolean }>("set_adzuna_credentials", { appId, appKey });
  },
  clearAdzunaCredentials: (): Promise<{ configured: boolean }> => {
    if (!isTauri()) return Promise.reject(new Error("Adzuna credentials are available only in the desktop app."));
    return invoke<{ configured: boolean }>("clear_adzuna_credentials");
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

  previewTestWorkflowReplacement: async (targetMode?: string): Promise<WorkflowReplacementPreview> => {
    if (isTauri()) return invoke<WorkflowReplacementPreview>("preview_test_workflow_replacement", targetMode ? { target_mode: targetMode } : {});
    return {
      target_template_ids: targetMode === "test_workflow"
        ? ["software-engineering-it-us-48h"]
        : SAMPLE_WORKFLOW_TEMPLATES.map(template => template.id),
      candidates: devSources.map(source => {
        const marker = demoWorkflowMarker(source);
        const legacy = LEGACY_DEMO_SAMPLE_IDS.has(source.id);
        const current = !!marker.template_id && exactDemoBuiltin(source, marker.template_id);
        return {
          id: source.id,
          name: source.name,
          fingerprint: demoReplacementFingerprint(source),
          selected_by_default: legacy,
          reason: legacy
            ? "Exact labeled legacy demonstration workflow"
            : current
              ? "Current built-in workflow — retained"
              : "Other workflow — preserved unless explicitly selected",
        };
      }),
    };
  },

  replaceTestWorkflows: async (selected: WorkflowReplacementSelection[], targetMode?: string): Promise<WorkflowReplacementReport> => {
    if (isTauri()) return invoke<WorkflowReplacementReport>("replace_test_workflows", targetMode ? { selected, target_mode: targetMode } : { selected });
    const preview = await api.previewTestWorkflowReplacement(targetMode);
    const seen = new Set<string>();
    for (const selection of selected) {
      if (seen.has(selection.id)) throw new Error("Duplicate workflow replacement selection.");
      seen.add(selection.id);
      const candidate = preview.candidates.find(item => item.id === selection.id);
      if (!candidate || candidate.fingerprint !== selection.fingerprint) {
        throw new Error("Workflow changed while replacing test workflows. Reload and review the selection.");
      }
    }
    const removed_ids = selected.map(selection => selection.id).sort();
    const preserved_ids = devSources.filter(source => !seen.has(source.id)).map(source => source.id).sort();
    const nextSources = devSources.filter(source => !seen.has(source.id));
    const created: Source[] = [];
    for (const template of SAMPLE_WORKFLOW_TEMPLATES) {
      if (nextSources.some(source => demoWorkflowMarker(source).template_id === template.id)) continue;
      const source = demoSourceFromTemplate(template.id, `demo-${template.id}-${crypto.randomUUID()}`);
      nextSources.push(source);
      created.push(source);
    }
    devSources = nextSources;
    saveDevSources();
    return {
      created,
      removed_ids,
      preserved_ids,
      backup_path: null,
      setup_required: ["Adzuna requires credentials in Settings and explicit source enablement."],
    };
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
        id: `source-${crypto.randomUUID()}`,
        kind: input.kind as any,
        name: input.name,
        identifier: input.identifier,
        config: (input.config || {}) as any,
        enabled: input.enabled ?? true,
        last_run_at: null,
        last_status: null,
        last_error: null,
        created_at: new Date().toISOString(),
        cadence: input.cadence || "daily",
        cadence_cron: input.cadence_cron || null,
        next_run_at: null,
        last_run_duration_ms: null,
        retention_days: BigInt(input.retention_days ?? 30),
        retention_policy: input.retention_policy || "untracked_only",
        filters: (input.filters || {}) as any,
        needs_review: false, script_trusted: false,
      };
      devSources.push(newSource);
      saveDevSources();
      return Promise.resolve(newSource);
    }
    return invoke<Source>("create_source", {
      input: {
        config: null,
        enabled: true,
        cadence: "daily",
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
          ...(input.config !== undefined ? { config: input.config as Source["config"] } : {}),
          ...(input.filters !== undefined ? { filters: input.filters as Source["filters"] } : {}),
          ...(input.name !== undefined && input.name !== null ? { name: input.name } : {}),
          ...(input.identifier !== undefined && input.identifier !== null ? { identifier: input.identifier } : {}),
          ...(input.enabled !== undefined && input.enabled !== null ? { enabled: input.enabled } : {}),
          ...(input.cadence !== undefined && input.cadence !== null ? { cadence: input.cadence } : {}),
          ...(input.cadence_cron !== undefined ? { cadence_cron: input.cadence_cron } : {}),
          ...(input.retention_days !== undefined && input.retention_days !== null ? { retention_days: BigInt(input.retention_days) } : {}),
          ...(input.retention_policy !== undefined && input.retention_policy !== null ? { retention_policy: input.retention_policy } : {}),
        };
        saveDevSources();
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
      saveDevSources();
      return Promise.resolve();
    }
    return invoke<void>("delete_source", { id });
  },

  // Scraping & Scheduler
  runScrapeNow: async (): Promise<ScrapeRun> => {
    if (!isTauri()) {
      const enabled = devSources.filter((s) => s.enabled);
      if (enabled.length) {
        const runs: ScrapeRun[] = [];
        for (const source of enabled) runs.push(await api.runSourceNow(source.id));
        const failed = runs.filter(run => !["success", "success_with_warnings"].includes(run.status));
        return { ...runs[runs.length - 1], trigger: "manual", source_id: null,
          status: failed.length ? "failed" : "success", error: failed.length ? failed.map(run => run.error || "Fixture run failed").join("; ") : null,
          jobs_found: runs.reduce((sum, run) => sum + run.jobs_found, 0n),
          jobs_new: runs.reduce((sum, run) => sum + run.jobs_new, 0n) };
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

  cancelScrape: (): Promise<boolean> => isTauri() ? invoke("cancel_scrape") : Promise.reject(new Error("Demonstration runs do not launch a worker process.")),

  runSourceNow: async (id: string): Promise<ScrapeRun> => {
    if (!isTauri()) {
      const target = devSources.find((s) => s.id === id);
      const runId = `mock-run-${crypto.randomUUID()}`;
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
          message: "Demonstration mode simulated a provider warning; no provider requests were made.",
        });
        await new Promise((r) => setTimeout(r, delayMs));

        const finishedAt = new Date().toISOString();
        const errMsg = "Simulated provider failure in demonstration mode; no network request was made.";
        devLogs[runId].push({
          id: `log-${Date.now()}-3`,
          run_id: runId,
          timestamp: finishedAt,
          level: "error",
          source_id: id,
          message: `Demonstration fixture failure: ${errMsg}`,
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
        message: "Demonstration mode generated fictional fixture postings; no provider requests were made.",
      });
      saveDevLogs();

      await new Promise((r) => setTimeout(r, delayMs));

      const newJobs = target
        ? generateScrapedJobsForSource(target)
        : generateScrapedJobsForSource({ ...SAMPLE_ENGINEERING_SOURCE, id, name: "Target Workflow" });

      let newlyAddedCount = 0;
      let refreshedCount = 0;
      const nowIso = new Date().toISOString();

      for (const candidate of newJobs) {
        if (devDeletedJobKeys.has(candidate.dedupe_key) || demoBlocks.some((b) => b.identity === demoIdentity(candidate))) {
          continue;
        }
        const existingIdx = devJobs.findIndex(
          (j) => j.dedupe_key === candidate.dedupe_key || j.url === candidate.url
        );
        if (existingIdx >= 0) {
          // Idempotent upsert: refresh last_seen_at and metadata, preserve user triage state!
          devJobs[existingIdx] = {
            ...candidate,
            id: devJobs[existingIdx].id, notes: devJobs[existingIdx].notes, status: devJobs[existingIdx].status, status_changed_at: devJobs[existingIdx].status_changed_at, favorited: devJobs[existingIdx].favorited, favorited_at: devJobs[existingIdx].favorited_at, date_added: devJobs[existingIdx].date_added,
            last_seen_at: nowIso,
            removed_at: null,
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

      const extractedCount = target ? generateScrapedJobsForSource(target, false).length : newJobs.length;
      const matchedCount = newJobs.length;

      devLogs[runId].push({
        id: `log-${Date.now()}-3`,
        run_id: runId,
        timestamp: new Date().toISOString(),
        level: "info",
        source_id: id,
        message: `Demonstration fixtures: ${extractedCount} fictional postings generated; ${matchedCount} matched the saved workflow.`,
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
        enabled_workflows: 0,
        schedules: [],
      });
    }
    return invoke<SchedulerStatus>("get_scheduler_status");
  },

  setSchedulerPaused: (paused: boolean) =>
    invoke<void>("set_scheduler_paused", { paused }),

  detectSource: async (url: string): Promise<DetectResult> => {
    if (isTauri()) return invoke<DetectResult>("detect_source", { url });
    const parsed = new URL(url);
    if (!["http:", "https:"].includes(parsed.protocol)) throw new Error("Use an HTTP(S) source URL");
    const kind = parsed.hostname.includes("greenhouse.io") ? "greenhouse" : parsed.hostname === "jobs.lever.co" ? "lever" : parsed.hostname === "jobs.ashbyhq.com" ? "ashby" : "jsonld";
    const identifier = kind === "jsonld" ? url : parsed.pathname.split("/").filter(Boolean)[0];
    if (!identifier) throw new Error("The URL must identify a board. Advanced supports custom selectors.");
    return { kind, identifier, name: kind === "jsonld" ? parsed.hostname : identifier };
  },
  describeWorkflowFilters: async (filters: WorkflowFilters): Promise<FilterCompatibilityRestriction[]> => {
    if (!isTauri()) throw new Error("Compatibility explanations require the desktop validator. Restrictions remain preserved; edit them directly or in YAML.");
    return invoke("describe_workflow_filters", { filters });
  },
  validateWorkflow: async (spec: unknown): Promise<WorkflowSpec> => {
    if (isTauri()) return invoke("validate_workflow", { spec });
    const result = validateWorkflowYaml(workflowSpecToYaml(spec as WorkflowSpec));
    if (!result.isValid) throw new Error(result.errors.map(e => e.message).join("; "));
    if (result.parsedSpec?.sources.some(source => {
      const config = source.extractor.source_config;
      const configuredMode = config && typeof config === "object" && !Array.isArray(config) ? config.mode : undefined;
      return source.extractor.mode === "script" || source.extractor.mode === "browser" || source.extractor.script || ["script", "browser"].includes(String(configuredMode));
    })) throw new Error("Browser and script extraction require the desktop application; executable configurations require local trust.");
    // Demonstration validates the form shape; expert syntax requires the desktop validator.
    if (result.parsedSpec?.filters?.query || result.parsedSpec?.filters?.policy) throw new Error("Expert queries and policies require desktop validation.");
    return { ...result.parsedSpec!, version: 9, max_jobs_per_run: result.parsedSpec!.max_jobs_per_run ?? INGESTION_LIMITS.default };
  },
  importWorkflow: async (spec: unknown): Promise<WorkflowSpec> => {
    if (isTauri()) return invoke("import_workflow", { spec });
    return api.validateWorkflow(spec);
  },
  trustWorkflowScript: async (sourceId: string, expectedHash: string, confirmed: boolean): Promise<Source> => {
    if (!isTauri()) throw new Error("Script trust requires the desktop application and local review.");
    return invoke("trust_workflow_script", { sourceId, expectedHash, confirmed });
  },
  saveWorkflow: async (spec: WorkflowSpec, id?: string, imported = false): Promise<Source> => {
    if (isTauri()) return invoke("save_workflow", { spec, id: id ?? null, imported });
    const validated = await api.validateWorkflow(spec);
    const input = workflowSpecToCreateInput(validated);
    return id ? api.updateSource({ ...input, id }) : api.createSource({ ...input, enabled: input.enabled ?? true, cadence: input.cadence ?? "daily" });
  },
  cancelWorkflowPreview: (requestId: string): Promise<boolean> => isTauri() ? invoke("cancel_workflow_preview", { requestId }) : Promise.resolve(true),
  previewWorkflow: async (spec: WorkflowSpec, requestId: string): Promise<WorkflowPreview> => {
    if (isTauri()) return invoke("preview_workflow", { spec, requestId });
    const validated = await api.validateWorkflow(spec);
    const input = workflowSpecToCreateInput(validated);
    const source = { ...SAMPLE_ENGINEERING_SOURCE, ...input, id: "demo-preview", created_at: new Date().toISOString() } as Source;
    const events = await api.testSourceTransient({ kind: source.kind, name: source.name, identifier: source.identifier, config: source.config, filters: {} });
    const fixtures = generateScrapedJobsForSource(source, false);
    const matched = events.filter(event => event.type === "job" && matchesDemoWorkflow(fixtures.find(job => job.external_id === event.external_id)!, source));
    const eligible = matched.filter(event => event.type === "job" && !demoBlocks.some(block => block.identity === fixtures.find(job => job.external_id === event.external_id)?.posting_identity));
    const previewLimit = Math.min(validated.max_jobs_per_run ?? INGESTION_LIMITS.default, INGESTION_LIMITS.preview);
    return { spec: validated, branding: [], jobs: eligible.slice(0, previewLimit).map(job => ({ identity: job.type === "job" ? `demo:${source.identifier}:${job.external_id}` : "", job })), fetched: events.length, matched: matched.length, rejected: events.length - matched.length, blocked: matched.length - eligible.length, invalid: 0, sampled: eligible.length >= previewLimit, errors: [], next_run_at: null };
  },

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
        location_data_complete: j.locations.some(
          (location) => Boolean(location.city || location.region_code || location.country_code),
        ),
        salary: j.salary_min
          ? {
              min: j.salary_min,
              max: j.salary_max,
              currency: j.salary_currency || "USD",
              interval: j.salary_interval || "hour",
              raw: j.salary_raw || "",
              source: "structured",
              ranges: j.compensation || [],
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
        ...SAMPLE_ENGINEERING_SOURCE,
        name: input.name,
        identifier: input.identifier,
        kind: input.kind as any,
        config: input.config as any,
        filters: input.filters,
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
        location_data_complete: j.locations.some(
          (location) => Boolean(location.city || location.region_code || location.country_code),
        ),
        salary: j.salary_min
          ? {
              min: j.salary_min,
              max: j.salary_max,
              currency: j.salary_currency || "USD",
              interval: j.salary_interval || "hour",
              raw: j.salary_raw || "",
              source: "structured",
              ranges: j.compensation || [],
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
  searchRoleTitles: (query: string): Promise<RoleTitleSuggestion[]> => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<RoleTitleSuggestion[]>("search_role_titles", { query });
  },

  searchCompanies: (query: string): Promise<CompanySuggestion[]> => {
    if (!isTauri()) return Promise.resolve([]);
    return invoke<CompanySuggestion[]>("search_companies", { query });
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

  detectBrowserRuntime: (): Promise<BrowserRuntimeStatus> => {
    if (!isTauri()) {
      return Promise.resolve({
        available: false,
        executable_path: null,
        version: null,
        message:
          "Browser extraction requires the desktop application and a local Chrome, Chromium, Brave, or Edge installation.",
      });
    }
    return invoke<BrowserRuntimeStatus>("detect_browser_runtime");
  },

  listBrowserSessions: (): Promise<BrowserSessionSummary[]> => {
    if (!isTauri()) {
      return Promise.resolve([]);
    }
    return invoke<BrowserSessionSummary[]>("list_browser_sessions");
  },

  openBrowserSession: (
    label: string,
    loginUrl: string,
  ): Promise<BrowserSessionSummary> => {
    if (!isTauri()) {
      return Promise.reject(
        new Error(
          "Browser login sessions require the desktop application.",
        ),
      );
    }
    return invoke<BrowserSessionSummary>("open_browser_session", {
      label,
      loginUrl,
    });
  },

  saveBrowserSession: (sessionId: string): Promise<BrowserSessionSummary> => {
    if (!isTauri()) {
      return Promise.reject(
        new Error(
          "Browser login sessions require the desktop application.",
        ),
      );
    }
    return invoke<BrowserSessionSummary>("save_browser_session", { sessionId });
  },

  clearBrowserSession: (sessionId: string): Promise<void> => {
    if (!isTauri()) {
      return Promise.reject(
        new Error(
          "Browser login sessions require the desktop application.",
        ),
      );
    }
    return invoke<void>("clear_browser_session", { sessionId });
  },

  testExtractor: (
    extractor: WorkflowExtractor,
  ): Promise<ExtractorTestResult> => {
    if (!isTauri()) {
      return Promise.reject(
        new Error(
          "Extractor testing requires the desktop application.",
        ),
      );
    }
    return invoke<ExtractorTestResult>("test_extractor", { extractor });
  },

  checkScriptEnvironment: (
    script: WorkflowScriptConfig,
  ): Promise<ScriptEnvironmentStatus> => {
    if (!isTauri()) {
      return Promise.resolve({
        available: true,
        runtime: script.runtime,
        executable_path: "/usr/local/bin/demo",
        version: "v20.0.0",
        missing_modules: [],
        message: null,
      });
    }
    return invoke<ScriptEnvironmentStatus>("check_script_environment", { script });
  },
};


// Browser execution is an explicit demo, never a silent production fallback.
export const api: typeof apiImplementation = new Proxy(apiImplementation, {
  get(target, key: keyof typeof apiImplementation) {
    const method = target[key];
    return (...args: unknown[]) => {
      if (isRecordedMode() && ["runSourceNow", "runScrapeNow", "previewWorkflow", "dryRunSource", "detectSource", "previewSource", "testSource", "testSourceTransient", "testExtractor"].includes(String(key))) return Promise.reject(new Error("Recorded examples cannot execute a scrape. Open the desktop application to test a live source."));
      if (needsDesktop()) return Promise.reject(new Error(
        "This action requires the desktop application. Open ?demo=1 for recorded provider examples."
      ));
      return Promise.resolve(Reflect.apply(method, target, args)).catch(error => {
        if (error instanceof Error) throw error;
        throw Object.assign(new Error(errorMessage(error)), error && typeof error === "object" ? error : {});
      });
    };
  },
});
