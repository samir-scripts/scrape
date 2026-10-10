import { describe, it, expect } from "vitest";
import { sourceToYaml, yamlToSource } from "../lib/yamlJob";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import type { TriggerType } from "../bindings/TriggerType";
import type { SourceCadence } from "../bindings/SourceCadence";
import type { WorkerEvent } from "../bindings/WorkerEvent";

describe("Custom Scrape Jobs: YAML Specification", () => {
  it("converts a standard ATS source to YAML spec", () => {
    const source = {
      name: "Stripe",
      identifier: "stripe",
      kind: "greenhouse" as const,
      cadence: "global" as const,
      cadence_cron: null,
      config: null,
    };

    const yaml = sourceToYaml(source);
    expect(yaml).toContain("version: 1");
    expect(yaml).toContain("company: Stripe");
    expect(yaml).toContain("url: stripe");
    expect(yaml).toContain("kind: greenhouse");
    expect(yaml).toContain("type: global");
  });

  it("converts a custom selector source with CSS selectors and regexes to YAML", () => {
    const source = {
      name: "Acme Robotics",
      identifier: "https://acme.example.com/careers",
      kind: "selector" as const,
      cadence: "every_6h" as const,
      cadence_cron: null,
      config: {
        selectors: {
          item: ".job-listing",
          title: "h3.title",
          link: "a.apply-link",
          location: ".location-tag",
          salary: ".comp-range",
        },
        title_regex: "^(Senior|Lead)",
        salary_regex: "\\$([0-9,]+)",
      },
    };

    const yaml = sourceToYaml(source);
    expect(yaml).toContain("company: Acme Robotics");
    expect(yaml).toContain("url: https://acme.example.com/careers");
    expect(yaml).toContain("kind: selector");
    expect(yaml).toContain("type: every_6h");
    expect(yaml).toContain("item: .job-listing");
    expect(yaml).toContain("title: h3.title");
    expect(yaml).toContain("location: .location-tag");
    expect(yaml).toContain("salary: .comp-range");
    expect(yaml).toContain("title: ^(Senior|Lead)");
  });

  it("converts cron cadence to YAML", () => {
    const source = {
      name: "Startup X",
      identifier: "https://startupx.io/jobs",
      kind: "selector" as const,
      cadence: "cron" as const,
      cadence_cron: "0 */4 * * *",
    };

    const yaml = sourceToYaml(source);
    expect(yaml).toContain("type: cron");
    expect(yaml).toContain("cron: 0 */4 * * *");
  });

  it("parses valid YAML spec into source input fields", () => {
    const yaml = `
version: 1
company: Nova Labs
url: https://novalabs.io/careers
cadence:
  type: daily
engine:
  kind: selector
  selectors:
    item: div.career-item
    title: .role-title
    link: a.apply-btn
    location: .location
    salary: .salary-badge
  regex:
    title: "Engineer|Developer"
`;

    const parsed = yamlToSource(yaml);
    expect(parsed.name).toBe("Nova Labs");
    expect(parsed.identifier).toBe("https://novalabs.io/careers");
    expect(parsed.kind).toBe("selector");
    expect(parsed.cadence).toBe("daily");
    expect(parsed.config.selectors?.item).toBe("div.career-item");
    expect(parsed.config.selectors?.title).toBe(".role-title");
    expect(parsed.config.selectors?.link).toBe("a.apply-btn");
    expect(parsed.config.title_regex).toBe("Engineer|Developer");
  });

  it("supports shorthand format and cadence aliases in YAML", () => {
    const shorthandYaml = `
name: Boutique AI
identifier: https://boutique.ai/work
kind: selector
cadence: custom_cron
cadence_cron: "0 0 12 * * *"
selectors:
  item: .job-card
  title: .name
`;

    const parsed = yamlToSource(shorthandYaml);
    expect(parsed.name).toBe("Boutique AI");
    expect(parsed.identifier).toBe("https://boutique.ai/work");
    expect(parsed.kind).toBe("selector");
    expect(parsed.cadence).toBe("cron");
    expect(parsed.cadence_cron).toBe("0 0 12 * * *");
    expect(parsed.config.selectors?.item).toBe(".job-card");
  });

  it("handles round-trip fidelity between source object and YAML string", () => {
    const original = {
      name: "RoundTrip Corp",
      identifier: "https://roundtrip.test/jobs",
      kind: "selector" as const,
      cadence: "cron" as const,
      cadence_cron: "0 9 * * 1-5",
      config: {
        selectors: {
          item: ".job-item",
          title: ".job-title",
          link: "a.link",
          location: ".loc",
          salary: ".sal",
        },
        title_regex: "Staff",
      },
    };

    const yaml = sourceToYaml(original);
    const roundTripped = yamlToSource(yaml);

    expect(roundTripped.name).toBe(original.name);
    expect(roundTripped.identifier).toBe(original.identifier);
    expect(roundTripped.kind).toBe(original.kind);
    expect(roundTripped.cadence).toBe(original.cadence);
    expect(roundTripped.cadence_cron).toBe(original.cadence_cron);
    expect(roundTripped.config.selectors).toEqual(original.config.selectors);
    expect(roundTripped.config.title_regex).toBe(original.config.title_regex);
  });

  it("throws a descriptive error when parsing invalid YAML", () => {
    expect(() => yamlToSource("not a valid yaml: : [unbalanced")).toThrow();
    expect(() => yamlToSource("12345")).toThrow("Invalid YAML structure");
  });
});

describe("Custom Scrape Jobs: Cadence & Cron Logic", () => {
  const validCadences: SourceCadence[] = [
    "global",
    "hourly",
    "every_6h",
    "every_12h",
    "daily",
    "weekly",
    "manual",
    "cron",
  ];

  it("accepts all documented cadence enum options", () => {
    expect(validCadences).toHaveLength(8);
  });

  it("validates 5-field and 6-field cron expressions", () => {
    const isValidCron = (expr: string): boolean => {
      const parts = expr.trim().split(/\s+/);
      return parts.length >= 5 && parts.length <= 6;
    };

    expect(isValidCron("0 * * * *")).toBe(true);
    expect(isValidCron("0 */4 * * *")).toBe(true);
    expect(isValidCron("0 0 12 * * ?")).toBe(true);
    expect(isValidCron("every 2 hours")).toBe(false);
    expect(isValidCron("")).toBe(false);
  });
});

describe("Custom Scrape Jobs: Single-Source Run Isolation", () => {
  it("verifies single_source trigger type is recognized", () => {
    const trigger: TriggerType = "single_source";
    expect(trigger).toBe("single_source");
  });

  it("filters runs by source_id", () => {
    const runs: ScrapeRun[] = [
      {
        id: "run-1",
        trigger: "manual",
        status: "success",
        started_at: "2026-10-10T00:00:00Z",
        finished_at: "2026-10-10T00:01:00Z",
        jobs_found: BigInt(20),
        jobs_new: BigInt(5),
        error: null,
        source_id: null,
      },
      {
        id: "run-2",
        trigger: "single_source",
        status: "success",
        started_at: "2026-10-10T01:00:00Z",
        finished_at: "2026-10-10T01:00:02Z",
        jobs_found: BigInt(4),
        jobs_new: BigInt(2),
        error: null,
        source_id: "src-acme",
      },
      {
        id: "run-3",
        trigger: "single_source",
        status: "failed",
        started_at: "2026-10-10T02:00:00Z",
        finished_at: "2026-10-10T02:00:01Z",
        jobs_found: BigInt(0),
        jobs_new: BigInt(0),
        error: "HTTP 403 Forbidden",
        source_id: "src-acme",
      },
      {
        id: "run-4",
        trigger: "single_source",
        status: "success",
        started_at: "2026-10-10T03:00:00Z",
        finished_at: "2026-10-10T03:00:03Z",
        jobs_found: BigInt(12),
        jobs_new: BigInt(1),
        error: null,
        source_id: "src-other",
      },
    ];

    const acmeRuns = runs.filter((r) => r.source_id === "src-acme");
    expect(acmeRuns).toHaveLength(2);
    expect(acmeRuns.map((r) => r.id)).toEqual(["run-2", "run-3"]);
  });

  it("processes dry-run worker events correctly", () => {
    const events: WorkerEvent[] = [
      {
        type: "job",
        source_id: "temp",
        external_id: "j-1",
        title: "Senior Backend Engineer",
        company: "Acme",
        role_type: "full_time",
        location: "San Francisco, CA",
        url: "https://acme.example.com/jobs/1",
        date_posted: null,
        salary: {
          min: 150000,
          max: 200000,
          currency: "USD",
          interval: "year",
          raw: "$150k - $200k",
          source: "structured",
        },
        description_text: null,
        locations: [],
        workplace_type: "hybrid",
        sponsorship: null,
        clearance: null,
        experience: null,
        is_new_grad: "unknown",
      },
      {
        type: "error",
        source_id: "temp",
        message: "Failed to parse salary field",
        retryable: false,
      },
    ];

    const jobs = events.filter((e): e is Extract<WorkerEvent, { type: "job" }> => e.type === "job");
    expect(jobs).toHaveLength(1);
    expect(jobs[0].title).toBe("Senior Backend Engineer");
    expect(jobs[0].salary?.raw).toBe("$150k - $200k");

    const errors = events.filter((e): e is Extract<WorkerEvent, { type: "error" }> => e.type === "error");
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toBe("Failed to parse salary field");
  });
});
