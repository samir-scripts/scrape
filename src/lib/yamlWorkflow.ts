import { INGESTION_LIMITS } from "../bindings/ingestionLimits";
import { parseDocument, stringify } from "yaml";
import type { SourceKind } from "../bindings/SourceKind";
import type { SourceCadence } from "../bindings/SourceCadence";
import type { RoleType } from "../bindings/RoleType";
import type { CreateSourceInput } from "../bindings/CreateSourceInput";
import type { Source } from "../bindings/Source";

import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import type { WorkflowSourceSpec } from "../bindings/WorkflowSourceSpec";
export type { WorkflowSpec } from "../bindings/WorkflowSpec";
export type { WorkflowSourceSpec } from "../bindings/WorkflowSourceSpec";
export type { WorkflowMetadata } from "../bindings/WorkflowMetadata";
export type { WorkflowExtractor } from "../bindings/WorkflowExtractor";
export type { WorkflowSchedule } from "../bindings/WorkflowSchedule";
export type { WorkflowFilters } from "../bindings/WorkflowFilters";
export type { WorkflowRetention } from "../bindings/WorkflowRetention";

export const SAMPLE_SWE_INTERNSHIP_YAML = `version: 6
metadata:
  name: "Cloudflare — Software Engineering Internships"
  company: "Cloudflare"
  enabled: true
sources:
  - id: "cloudflare-swe"
    label: "Cloudflare Greenhouse"
    enabled: true
    employer_hint: "Cloudflare"
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/cloudflare"
schedule:
  cadence: daily
  time: "09:00"
  cron: "0 0 9 * * *"
filters:
  role_types: [internship]
  include_keywords: ["Software", "Engineer", "Intern"]
  exclude_keywords: ["MBA", "Sales", "Marketing"]
  locations: ["Remote", "San Francisco, CA", "Austin, TX"]
retention:
  days: 45
  policy: untracked_only
`;

export const SAMPLE_SWE_INTERNSHIP_SPEC: WorkflowSpec = {
  version: 6,
  metadata: {
    name: "Cloudflare — Software Engineering Internships",
    company: "Cloudflare",
    enabled: true,
  },
  sources: [
    {
      id: "cloudflare-swe",
      label: "Cloudflare Greenhouse",
      enabled: true,
      employer_hint: "Cloudflare",
      extractor: {
        kind: "greenhouse",
        url: "https://boards.greenhouse.io/cloudflare",
      },
    },
  ],
  schedule: {
    cadence: "daily",
    time: "09:00",
    cron: "0 0 9 * * *",
  },
  filters: {
    role_types: ["internship"],
    include_keywords: ["Software", "Engineer", "Intern"],
    exclude_keywords: ["MBA", "Sales", "Marketing"],
    locations: ["Remote", "San Francisco, CA", "Austin, TX"],
  },
  retention: {
    days: 45,
    policy: "untracked_only",
  },
};

export interface YamlValidationError {
  line?: number;
  col?: number;
  message: string;
}

export interface YamlValidationResult {
  isValid: boolean;
  errors: YamlValidationError[];
  parsedSpec?: WorkflowSpec;
}

const VALID_KINDS: SourceKind[] = [
  "greenhouse",
  "lever",
  "ashby",
  "jsonld",
  "selector",
];
const VALID_CADENCES: SourceCadence[] = [
  "global",
  "hourly",
  "every_6h",
  "every_12h",
  "daily",
  "weekly",
  "manual",
  "cron",
];
const VALID_ROLE_TYPES: RoleType[] = [
  "full_time",
  "contract",
  "internship",
  "part_time",
  "seasonal",
  "unknown",
];

export function normalizedEndpoint(kind: SourceKind, urlOrSlug: string): string {
  const trimmed = (urlOrSlug || "").trim();
  if (["greenhouse", "lever", "ashby"].includes(kind)) {
    if (!trimmed.includes("://")) {
      return `${kind}:${trimmed.toLowerCase()}`;
    }
    try {
      const parsed = new URL(trimmed);
      const slug = parsed.pathname.split("/").filter(Boolean)[0] || "";
      return `${kind}:${slug.toLowerCase()}`;
    } catch {
      return `${kind}:${trimmed.toLowerCase()}`;
    }
  }
  try {
    const parsed = new URL(trimmed);
    parsed.hash = "";
    parsed.search = "";
    let pathname = parsed.pathname.toLowerCase();
    while (pathname.endsWith("/") && pathname.length > 1) {
      pathname = pathname.slice(0, -1);
    }
    return `${kind}:${parsed.protocol}//${parsed.host.toLowerCase()}${pathname}`;
  } catch {
    return `${kind}:${trimmed.toLowerCase()}`;
  }
}

export function validateWorkflowYaml(
  yamlContent: string,
): YamlValidationResult {
  if (!yamlContent.trim()) {
    return {
      isValid: false,
      errors: [{ message: "Workflow specification is empty" }],
    };
  }

  const doc = parseDocument(yamlContent);
  const errors: YamlValidationError[] = [];

  if (doc.errors && doc.errors.length > 0) {
    for (const err of doc.errors) {
      const linePos = (
        err as unknown as { linePos?: [{ line: number; col: number }] }
      ).linePos;
      const line = linePos?.[0]?.line;
      const col = linePos?.[0]?.col;
      errors.push({
        line,
        col,
        message: err.message || "YAML Syntax Error",
      });
    }
    return {
      isValid: false,
      errors,
    };
  }

  const json = doc.toJSON() as Partial<WorkflowSpec>;

  if (typeof json !== "object" || json === null || Array.isArray(json)) {
    return {
      isValid: false,
      errors: [
        { message: "Top-level specification must be an object/mapping" },
      ],
    };
  }

  // Version check
  if (json.version !== undefined && ![1, 2, 3, 4, 5, 6].includes(json.version)) {
    errors.push({
      message: `Unsupported workflow version ${json.version}. Expected supported versions: 1–6`,
    });
  }

  if (json.max_jobs_per_run != null && (!Number.isInteger(json.max_jobs_per_run) || json.max_jobs_per_run < 1 || json.max_jobs_per_run > INGESTION_LIMITS.maximum)) {
    errors.push({ message: `max_jobs_per_run: enter a whole number from 1 to ${INGESTION_LIMITS.maximum.toLocaleString("en-US")}` });
  }

  // Metadata check
  if (!json.metadata || typeof json.metadata !== "object") {
    errors.push({ message: "Missing required 'metadata' section" });
  } else {
    if (
      !json.metadata.name ||
      typeof json.metadata.name !== "string" ||
      !json.metadata.name.trim()
    ) {
      errors.push({ message: "metadata.name must be a non-empty string" });
    }
  }

  // Migrate legacy extractor to sources if present
  const rawJson = json as Record<string, unknown>;
  if (rawJson.extractor && (!rawJson.sources || !Array.isArray(rawJson.sources))) {
    rawJson.sources = [
      {
        id: "default",
        label: typeof json.metadata?.name === "string" && json.metadata.name.trim() ? json.metadata.name.trim() : "Default source",
        enabled: true,
        extractor: rawJson.extractor,
      },
    ];
    delete rawJson.extractor;
  }

  // Sources check
  if (!rawJson.sources || !Array.isArray(rawJson.sources)) {
    errors.push({ message: "Missing required 'sources' list" });
  } else if (rawJson.sources.length === 0) {
    errors.push({ message: "sources: must contain at least one source" });
  } else {
    const sources = rawJson.sources as Array<Partial<WorkflowSourceSpec>>;
    if (!sources.some(s => s.enabled !== false)) {
      errors.push({ message: "sources: at least one source must be enabled" });
    }
    const seenIds = new Set<string>();
    const seenEndpoints = new Set<string>();

    for (let i = 0; i < sources.length; i++) {
      const src = sources[i];
      if (!src || typeof src !== "object") {
        errors.push({ message: `sources[${i}]: must be an object` });
        continue;
      }
      const id = typeof src.id === "string" ? src.id.trim() : "";
      if (!id) {
        errors.push({ message: `sources[${i}].id: must not be blank` });
      } else if (seenIds.has(id)) {
        errors.push({ message: `sources.id: duplicate source ID '${id}'` });
      } else {
        seenIds.add(id);
      }

      const label = typeof src.label === "string" ? src.label.trim() : "";
      if (!label) {
        errors.push({ message: `sources[${i}].label: must not be blank` });
      }

      if (!src.extractor || typeof src.extractor !== "object") {
        errors.push({ message: `sources[${i}].extractor: missing required extractor` });
      } else {
        const extractor = src.extractor;
        if (!extractor.kind || !VALID_KINDS.includes(extractor.kind)) {
          errors.push({
            message: `sources[${i}].extractor.kind must be one of: ${VALID_KINDS.join(", ")}`,
          });
        }
        if (
          !extractor.url ||
          typeof extractor.url !== "string" ||
          !extractor.url.trim()
        ) {
          errors.push({
            message: `sources[${i}].extractor.url must be a valid non-empty URL string`,
          });
        } else {
          const isAtsKind = ["greenhouse", "lever", "ashby"].includes(
            extractor.kind,
          );
          const isPlainSlug =
            isAtsKind && /^[a-zA-Z0-9_\-\.]+$/.test(extractor.url.trim());
          if (!isPlainSlug) {
            try {
              const u = new URL(extractor.url);
              if (u.protocol !== "http:" && u.protocol !== "https:") {
                errors.push({
                  message: `sources[${i}].extractor.url must start with http:// or https://`,
                });
              }
            } catch {
              errors.push({ message: `sources[${i}].extractor.url is not a valid URL` });
            }
          }

          if (extractor.kind) {
            const endpointKey = normalizedEndpoint(extractor.kind, extractor.url);
            if (seenEndpoints.has(endpointKey)) {
              errors.push({
                message: `Duplicate source endpoint: '${extractor.url}' conflicts with another source in this workflow`,
              });
            } else {
              seenEndpoints.add(endpointKey);
            }
          }
        }
      }
    }
  }

  // Schedule check
  if (json.schedule && typeof json.schedule === "object") {
    if (
      json.schedule.cadence &&
      !VALID_CADENCES.includes(json.schedule.cadence)
    ) {
      errors.push({
        message: `schedule.cadence must be one of: ${VALID_CADENCES.join(", ")}`,
      });
    }
    if (
      json.schedule.cadence === "cron" &&
      (!json.schedule.cron || !json.schedule.cron.trim())
    ) {
      errors.push({
        message: "schedule.cron is required when cadence is set to 'cron'",
      });
    }
  }

  // Retention check
  if (json.retention && typeof json.retention === "object") {
    if (
      json.retention.days !== undefined &&
      (typeof json.retention.days !== "number" || json.retention.days < 0)
    ) {
      errors.push({
        message:
          "retention.days must be a non-negative number (0 = keep forever)",
      });
    }
    if (
      json.retention.policy !== undefined &&
      json.retention.policy !== "untracked_only" &&
      json.retention.policy !== "all"
    ) {
      errors.push({
        message: "retention.policy must be either 'untracked_only' or 'all'",
      });
    }
  }

  // Filters check
  if (json.filters && typeof json.filters === "object") {
    if (json.filters.role_types != null) {
      if (!Array.isArray(json.filters.role_types)) {
        errors.push({ message: "filters.role_types must be an array" });
      } else {
        for (const rt of json.filters.role_types) {
          if (!VALID_ROLE_TYPES.includes(rt)) {
            errors.push({
              message: `Invalid role type '${rt}'. Allowed: ${VALID_ROLE_TYPES.join(", ")}`,
            });
          }
        }
      }
    }

    if (json.filters.query != null) {
      if (typeof json.filters.query !== "string") {
        errors.push({ message: "filters.query must be a string expression" });
      } else {
        const q = json.filters.query.trim();
        // Check for basic query syntax errors (unmatched parens, unclosed quotes)
        let parenCount = 0;
        let inQuote: string | null = null;
        for (let i = 0; i < q.length; i++) {
          const char = q[i];
          if (char === "'" || char === '"') {
            if (inQuote === char) inQuote = null;
            else if (!inQuote) inQuote = char;
          } else if (!inQuote) {
            if (char === "(") parenCount++;
            else if (char === ")") {
              parenCount--;
              if (parenCount < 0) {
                errors.push({
                  message: `Unmatched closing parenthesis ')' at character ${i + 1}`,
                });
                break;
              }
            }
          }
        }
        if (inQuote) {
          errors.push({
            message: `Unclosed string quote (${inQuote}) in filter query`,
          });
        }
        if (parenCount > 0) {
          errors.push({
            message: `Unclosed opening parenthesis '(' in filter query`,
          });
        }
      }
    }

    if (
      json.filters.policy !== undefined &&
      (typeof json.filters.policy !== "object" || json.filters.policy === null)
    ) {
      errors.push({
        message: "filters.policy must be a valid policy object mapping",
      });
    }
  }

  if (errors.length > 0) {
    return {
      isValid: false,
      errors,
    };
  }

  return {
    isValid: true,
    errors: [],
    parsedSpec: { ...json, version: json.version ?? 6 } as WorkflowSpec,
  };
}

export function workflowSpecToYaml(spec: WorkflowSpec): string {
  return stringify(spec, { indent: 2, lineWidth: 0 });
}

export function sourceToWorkflowSpec(source: Source): WorkflowSpec {
  const config = (
    source.config &&
    typeof source.config === "object" &&
    !Array.isArray(source.config)
      ? source.config
      : {}
  ) as Record<string, unknown>;
  // needs_review covers two states. Unconvertible originals (no fingerprint)
  // keep ignoring their possibly invalid snapshot. Script sources awaiting trust
  // have a fingerprint written only after Rust validation, so their snapshot is
  // authoritative and must be editable and reviewable.
  const awaitingScriptTrust = !!source.script_sha256 && !source.script_trusted;
  const snapshot = source.needs_review && !awaitingScriptTrust
    ? undefined
    : (config.__workflow_spec as WorkflowSpec | undefined);
  const sourceConfig = Object.fromEntries(
    Object.entries(config).filter(([key]) => !key.startsWith("__")),
  );
  const selectors = {
    job_container: "item",
    title_selector: "title",
    url_selector: "link",
    location_selector: "location",
    salary_selector: "salary",
    description_selector: "description",
    date_selector: "date",
    employer_logo_selector: "employer_logo",
    title_regex: "title_regex",
    link_regex: "link_regex",
    location_regex: "location_regex",
    salary_regex: "salary_regex",
    url_pattern: "url_pattern",
    url_patterns: "url_patterns",
    exclude_url_patterns: "exclude_url_patterns",
  };
  const extractedFields = Object.fromEntries(
    Object.entries(selectors)
      .filter(([, key]) => key in sourceConfig)
      .map(([field, key]) => [field, sourceConfig[key]]),
  );
  for (const key of Object.values(selectors)) delete sourceConfig[key];
  const filters = (source.filters || {}) as WorkflowSpec["filters"];
  const calendar = config.__schedule as
    { time?: string; weekdays?: number[]; timezone?: string } | undefined;
  const runOnStartup = config.__run_on_startup;

  const sources: WorkflowSourceSpec[] = snapshot?.sources?.length
    ? snapshot.sources
    : [
        {
          id: source.id,
          label: source.name,
          enabled: source.enabled,
          employer_hint: (config.__company as string) || undefined,
          extractor: {
            ...extractedFields,
            kind: source.kind,
            url: String(canonicalizeAtsUrl(source.kind, source.identifier)),
            source_config: sourceConfig as WorkflowSourceSpec["extractor"]["source_config"],
          },
        },
      ];

  return {
    ...snapshot,
    version: 6,
    max_jobs_per_run: snapshot?.max_jobs_per_run ?? INGESTION_LIMITS.default,
    metadata: {
      ...snapshot?.metadata,
      name: source.name,
      company: (config.__company as string) || snapshot?.metadata?.company,
      enabled: source.enabled,
    },
    sources,
    schedule: {
      ...snapshot?.schedule,
      ...(typeof config.__cron_timezone === "string"
        ? { timezone: config.__cron_timezone }
        : {}),
      cadence: source.cadence === "global" ? "daily" : source.cadence,
      cron: source.cadence_cron ?? undefined,
      ...calendar,
      run_on_startup: snapshot?.schedule?.run_on_startup ?? (typeof runOnStartup === "boolean" ? runOnStartup : false),
    },
    filters,
    retention: {
      days: Number(source.retention_days),
      policy: source.retention_policy,
    },
  };
}

export function workflowSpecToCreateInput(
  spec: WorkflowSpec,
): CreateSourceInput {
  const primarySource = spec.sources.find((s) => s.enabled) || spec.sources[0] || {
    id: "default",
    label: spec.metadata.name,
    enabled: true,
    extractor: { kind: "jsonld" as SourceKind, url: "" },
  };
  let identifier = primarySource.extractor.url;
  if (
    ["greenhouse", "lever", "ashby"].includes(primarySource.extractor.kind) &&
    identifier.includes("://")
  )
    identifier =
      new URL(identifier).pathname.split("/").filter(Boolean)[0] || "";
  const config: Record<string, unknown> = {
    ...((primarySource.extractor.source_config as Record<string, unknown>) || {}),
    url: primarySource.extractor.url,
    __workflow_spec: spec,
  };
  const mapping = {
    job_container: "item",
    title_selector: "title",
    url_selector: "link",
    location_selector: "location",
    salary_selector: "salary",
    description_selector: "description",
    date_selector: "date",
    employer_logo_selector: "employer_logo",
    title_regex: "title_regex",
    link_regex: "link_regex",
    location_regex: "location_regex",
    salary_regex: "salary_regex",
    url_pattern: "url_pattern",
    url_patterns: "url_patterns",
    exclude_url_patterns: "exclude_url_patterns",
  } as const;
  for (const [field, key] of Object.entries(mapping)) {
    const value = primarySource.extractor[field as keyof typeof mapping];
    if (value != null) config[key] = value;
  }
  const cadence = spec.schedule?.cadence || "daily";
  if (["daily", "weekly"].includes(cadence))
    config.__schedule = {
      time: spec.schedule?.time || "09:00",
      weekdays: spec.schedule?.weekdays || (cadence === "weekly" ? [1] : []),
      timezone: spec.schedule?.timezone || "UTC",
    };
  if (spec.metadata.company) config.__company = spec.metadata.company;
  return {
    kind: primarySource.extractor.kind,
    name: spec.metadata.name,
    identifier,
    config: config as CreateSourceInput["config"],
    enabled: spec.metadata.enabled ?? true,
    cadence,
    cadence_cron: spec.schedule?.cron ?? null,
    retention_days: BigInt(spec.retention?.days ?? 30),
    retention_policy: spec.retention?.policy || "untracked_only",
    filters: (spec.filters || {}) as CreateSourceInput["filters"],
  };
}

export function canonicalizeAtsUrl(kind: string, urlOrSlug: string): string {
  const trimmed = (urlOrSlug || "").trim();
  if (!trimmed) return "";
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    return trimmed;
  }
  if (kind === "greenhouse") return `https://boards.greenhouse.io/${trimmed}`;
  if (kind === "lever") return `https://jobs.lever.co/${trimmed}`;
  if (kind === "ashby") return `https://jobs.ashbyhq.com/${trimmed}`;
  return trimmed;
}
