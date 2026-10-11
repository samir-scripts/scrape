import { parseDocument } from "yaml";
import type { SourceKind } from "../bindings/SourceKind";
import type { SourceCadence } from "../bindings/SourceCadence";
import type { RoleType } from "../bindings/RoleType";
import type { CreateSourceInput } from "../bindings/CreateSourceInput";
import type { Source } from "../bindings/Source";

export interface WorkflowMetadata {
  name: string;
  company?: string;
  enabled?: boolean;
}

export interface WorkflowExtractor {
  kind: SourceKind;
  url: string;
  job_container?: string;
  title_selector?: string;
  url_selector?: string;
  location_selector?: string;
  date_selector?: string;
  salary_selector?: string;
  description_selector?: string;
  salary_regex?: string;
  experience_regex?: string;
  clearance_regex?: string;
  sponsorship_regex?: string;
  url_pattern?: string;
  url_patterns?: string[];
  exclude_url_patterns?: string[];
}

export interface WorkflowSchedule {
  cadence?: SourceCadence;
  time?: string;
  cron?: string;
}

export interface WorkflowFilters {
  role_types?: RoleType[];
  include_keywords?: string[];
  exclude_keywords?: string[];
  locations?: string[];
  experience_min?: number;
  experience_max?: number;
}

export interface WorkflowRetention {
  days?: number;
  policy?: "untracked_only" | "all";
}

export interface WorkflowSpec {
  version: 2;
  metadata: WorkflowMetadata;
  extractor: WorkflowExtractor;
  schedule?: WorkflowSchedule;
  filters?: WorkflowFilters;
  retention?: WorkflowRetention;
}

export const SAMPLE_SWE_INTERNSHIP_YAML = `version: 2
metadata:
  name: "Cloudflare — Software Engineering Internships"
  company: "Cloudflare"
  enabled: true
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
  version: 2,
  metadata: {
    name: "Cloudflare — Software Engineering Internships",
    company: "Cloudflare",
    enabled: true,
  },
  extractor: {
    kind: "greenhouse",
    url: "https://boards.greenhouse.io/cloudflare",
  },
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

const VALID_KINDS: SourceKind[] = ["greenhouse", "lever", "ashby", "jsonld", "selector"];
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
const VALID_ROLE_TYPES: RoleType[] = ["full_time", "contract", "internship", "part_time", "seasonal", "unknown"];

export function validateWorkflowYaml(yamlContent: string): YamlValidationResult {
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
      const linePos = (err as unknown as { linePos?: [ { line: number; col: number } ] }).linePos;
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
      errors: [{ message: "Top-level specification must be an object/mapping" }],
    };
  }

  // Version check
  if (json.version !== undefined && json.version !== 2) {
    errors.push({ message: `Unsupported workflow version ${json.version}. Expected version: 2` });
  }

  // Metadata check
  if (!json.metadata || typeof json.metadata !== "object") {
    errors.push({ message: "Missing required 'metadata' section" });
  } else {
    if (!json.metadata.name || typeof json.metadata.name !== "string" || !json.metadata.name.trim()) {
      errors.push({ message: "metadata.name must be a non-empty string" });
    }
  }

  // Extractor check
  if (!json.extractor || typeof json.extractor !== "object") {
    errors.push({ message: "Missing required 'extractor' section" });
  } else {
    if (!json.extractor.kind || !VALID_KINDS.includes(json.extractor.kind)) {
      errors.push({
        message: `extractor.kind must be one of: ${VALID_KINDS.join(", ")}`,
      });
    }
    if (!json.extractor.url || typeof json.extractor.url !== "string" || !json.extractor.url.trim()) {
      errors.push({ message: "extractor.url must be a valid non-empty URL string" });
    } else {
      const isAtsKind = ["greenhouse", "lever", "ashby"].includes(json.extractor.kind);
      const isPlainSlug = isAtsKind && /^[a-zA-Z0-9_\-\.]+$/.test(json.extractor.url.trim());
      if (!isPlainSlug) {
        try {
          const u = new URL(json.extractor.url);
          if (u.protocol !== "http:" && u.protocol !== "https:") {
            errors.push({ message: "extractor.url must start with http:// or https://" });
          }
        } catch {
          errors.push({ message: "extractor.url is not a valid URL" });
        }
      }
    }
  }

  // Schedule check
  if (json.schedule && typeof json.schedule === "object") {
    if (json.schedule.cadence && !VALID_CADENCES.includes(json.schedule.cadence)) {
      errors.push({
        message: `schedule.cadence must be one of: ${VALID_CADENCES.join(", ")}`,
      });
    }
    if (json.schedule.cadence === "cron" && (!json.schedule.cron || !json.schedule.cron.trim())) {
      errors.push({ message: "schedule.cron is required when cadence is set to 'cron'" });
    }
  }

  // Retention check
  if (json.retention && typeof json.retention === "object") {
    if (json.retention.days !== undefined && (typeof json.retention.days !== "number" || json.retention.days < 0)) {
      errors.push({ message: "retention.days must be a non-negative number (0 = keep forever)" });
    }
    if (json.retention.policy !== undefined && json.retention.policy !== "untracked_only" && json.retention.policy !== "all") {
      errors.push({ message: "retention.policy must be either 'untracked_only' or 'all'" });
    }
  }

  // Filters check
  if (json.filters && typeof json.filters === "object") {
    if (json.filters.role_types !== undefined) {
      if (!Array.isArray(json.filters.role_types)) {
        errors.push({ message: "filters.role_types must be an array" });
      } else {
        for (const rt of json.filters.role_types) {
          if (!VALID_ROLE_TYPES.includes(rt)) {
            errors.push({ message: `Invalid role type '${rt}'. Allowed: ${VALID_ROLE_TYPES.join(", ")}` });
          }
        }
      }
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
    parsedSpec: json as WorkflowSpec,
  };
}

export function workflowSpecToYaml(spec: WorkflowSpec): string {
  const lines: string[] = [];

  lines.push("version: 2");
  lines.push("metadata:");
  lines.push(`  name: "${spec.metadata.name.replace(/"/g, '\\"')}"`);
  if (spec.metadata.company) {
    lines.push(`  company: "${spec.metadata.company.replace(/"/g, '\\"')}"`);
  }
  lines.push(`  enabled: ${spec.metadata.enabled ?? true}`);

  lines.push("extractor:");
  lines.push(`  kind: ${spec.extractor.kind}`);
  lines.push(`  url: "${spec.extractor.url}"`);
  if (spec.extractor.job_container) lines.push(`  job_container: "${spec.extractor.job_container}"`);
  if (spec.extractor.title_selector) lines.push(`  title_selector: "${spec.extractor.title_selector}"`);
  if (spec.extractor.url_selector) lines.push(`  url_selector: "${spec.extractor.url_selector}"`);
  if (spec.extractor.location_selector) lines.push(`  location_selector: "${spec.extractor.location_selector}"`);
  if (spec.extractor.salary_selector) lines.push(`  salary_selector: "${spec.extractor.salary_selector}"`);
  if (spec.extractor.url_pattern) lines.push(`  url_pattern: "${spec.extractor.url_pattern}"`);
  if (spec.extractor.url_patterns && spec.extractor.url_patterns.length > 0) {
    lines.push(`  url_patterns: [${spec.extractor.url_patterns.map((p) => `"${p}"`).join(", ")}]`);
  }
  if (spec.extractor.exclude_url_patterns && spec.extractor.exclude_url_patterns.length > 0) {
    lines.push(`  exclude_url_patterns: [${spec.extractor.exclude_url_patterns.map((p) => `"${p}"`).join(", ")}]`);
  }

  lines.push("schedule:");
  lines.push(`  cadence: ${spec.schedule?.cadence ?? "global"}`);
  if (spec.schedule?.time) lines.push(`  time: "${spec.schedule.time}"`);
  if (spec.schedule?.cron) lines.push(`  cron: "${spec.schedule.cron}"`);

  lines.push("filters:");
  if (spec.filters?.role_types && spec.filters.role_types.length > 0) {
    lines.push(`  role_types: [${spec.filters.role_types.join(", ")}]`);
  } else {
    lines.push("  role_types: []");
  }
  if (spec.filters?.include_keywords && spec.filters.include_keywords.length > 0) {
    lines.push(`  include_keywords: [${spec.filters.include_keywords.map((k) => `"${k}"`).join(", ")}]`);
  }
  if (spec.filters?.exclude_keywords && spec.filters.exclude_keywords.length > 0) {
    lines.push(`  exclude_keywords: [${spec.filters.exclude_keywords.map((k) => `"${k}"`).join(", ")}]`);
  }
  if (spec.filters?.locations && spec.filters.locations.length > 0) {
    lines.push(`  locations: [${spec.filters.locations.map((l) => `"${l}"`).join(", ")}]`);
  }

  lines.push("retention:");
  lines.push(`  days: ${spec.retention?.days ?? 30}`);
  lines.push(`  policy: ${spec.retention?.policy ?? "untracked_only"}`);

  return lines.join("\n") + "\n";
}

export function sourceToWorkflowSpec(source: Source): WorkflowSpec {
  const config = (typeof source.config === "object" && source.config !== null ? source.config : {}) as Record<string, unknown>;
  const filters = (typeof source.filters === "object" && source.filters !== null ? source.filters : {}) as Record<string, unknown>;

  const url = (config.url as string) || (source.kind === "greenhouse" ? `https://boards.greenhouse.io/${source.identifier}` : source.identifier);

  return {
    version: 2,
    metadata: {
      name: source.name,
      company: source.name,
      enabled: source.enabled,
    },
    extractor: {
      kind: source.kind,
      url,
      job_container: config.job_container as string | undefined,
      title_selector: config.title_selector as string | undefined,
      url_selector: config.url_selector as string | undefined,
      location_selector: config.location_selector as string | undefined,
      salary_selector: config.salary_selector as string | undefined,
      url_pattern: config.url_pattern as string | undefined,
      url_patterns: config.url_patterns as string[] | undefined,
      exclude_url_patterns: config.exclude_url_patterns as string[] | undefined,
    },
    schedule: {
      cadence: source.cadence,
      cron: source.cadence_cron ?? undefined,
    },
    filters: {
      role_types: filters.role_types as RoleType[] | undefined,
      include_keywords: filters.include_keywords as string[] | undefined,
      exclude_keywords: filters.exclude_keywords as string[] | undefined,
      locations: filters.locations as string[] | undefined,
    },
    retention: {
      days: Number(source.retention_days ?? 30),
      policy: (source.retention_policy as "untracked_only" | "all") ?? "untracked_only",
    },
  };
}

export function workflowSpecToCreateInput(spec: WorkflowSpec): CreateSourceInput {
  let identifier = spec.extractor.url;
  if (spec.extractor.kind === "greenhouse") {
    const match = spec.extractor.url.match(/boards\.greenhouse\.io\/([^/?#]+)/i);
    if (match) identifier = match[1];
  } else if (spec.extractor.kind === "lever") {
    const match = spec.extractor.url.match(/jobs\.lever\.co\/([^/?#]+)/i);
    if (match) identifier = match[1];
  } else if (spec.extractor.kind === "ashby") {
    const match = spec.extractor.url.match(/jobs\.ashbyhq\.com\/([^/?#]+)/i);
    if (match) identifier = match[1];
  }

  const configObj: Record<string, unknown> = {
    url: spec.extractor.url,
  };
  if (spec.extractor.job_container) configObj.job_container = spec.extractor.job_container;
  if (spec.extractor.title_selector) configObj.title_selector = spec.extractor.title_selector;
  if (spec.extractor.url_selector) configObj.url_selector = spec.extractor.url_selector;
  if (spec.extractor.location_selector) configObj.location_selector = spec.extractor.location_selector;
  if (spec.extractor.salary_selector) configObj.salary_selector = spec.extractor.salary_selector;
  if (spec.extractor.url_pattern) configObj.url_pattern = spec.extractor.url_pattern;
  if (spec.extractor.url_patterns) configObj.url_patterns = spec.extractor.url_patterns;
  if (spec.extractor.exclude_url_patterns) configObj.exclude_url_patterns = spec.extractor.exclude_url_patterns;

  const filtersObj: Record<string, unknown> = {};
  if (spec.filters?.role_types) filtersObj.role_types = spec.filters.role_types;
  if (spec.filters?.include_keywords) filtersObj.include_keywords = spec.filters.include_keywords;
  if (spec.filters?.exclude_keywords) filtersObj.exclude_keywords = spec.filters.exclude_keywords;
  if (spec.filters?.locations) filtersObj.locations = spec.filters.locations;

  return {
    kind: spec.extractor.kind,
    name: spec.metadata.name,
    identifier,
    config: configObj as unknown as CreateSourceInput["config"],
    enabled: spec.metadata.enabled ?? true,
    cadence: spec.schedule?.cadence ?? "global",
    cadence_cron: spec.schedule?.cron ?? null,
    retention_days: BigInt(spec.retention?.days ?? 30),
    retention_policy: spec.retention?.policy ?? "untracked_only",
    filters: filtersObj as unknown as CreateSourceInput["filters"],
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
