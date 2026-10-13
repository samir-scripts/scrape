import { describe, it, expect } from "vitest";
import {
  validateWorkflowYaml,
  workflowSpecToYaml,
  workflowSpecToCreateInput,
  sourceToWorkflowSpec,
  canonicalizeAtsUrl,
  type WorkflowSpec,
} from "../lib/yamlWorkflow";
import type { Source } from "../bindings/Source";

describe("YAML Workflow Validator & Serializer", () => {
  const validV2Yaml = `
version: 2
metadata:
  name: "Stripe — Core Infrastructure"
  company: "Stripe"
  enabled: true
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
schedule:
  cadence: daily
  time: "09:00"
  cron: "0 0 9 * * *"
filters:
  role_types: [full_time, contract]
  include_keywords: ["Software Engineer", "Rust"]
  exclude_keywords: ["Manager"]
  locations: ["Remote", "San Francisco, CA"]
retention:
  days: 30
  policy: untracked_only
`;

  it("validates a fully-specified v2 YAML workflow", () => {
    const result = validateWorkflowYaml(validV2Yaml);
    expect(result.isValid).toBe(true);
    expect(result.errors).toHaveLength(0);
    expect(result.parsedSpec).toBeDefined();

    const spec = result.parsedSpec!;
    expect(spec.version).toBe(2);
    expect(spec.metadata.name).toBe("Stripe — Core Infrastructure");
    expect(spec.extractor.kind).toBe("greenhouse");
    expect(spec.extractor.url).toBe("https://boards.greenhouse.io/stripe");
    expect(spec.schedule?.cadence).toBe("daily");
    expect(spec.retention?.days).toBe(30);
    expect(spec.retention?.policy).toBe("untracked_only");
    expect(spec.filters?.role_types).toEqual(["full_time", "contract"]);
  });

  it("catches YAML syntax errors and reports syntax failure", () => {
    const invalidSyntaxYaml = `
version: 2
metadata:
  name: [unclosed array
extractor:
  kind: greenhouse
`;
    const result = validateWorkflowYaml(invalidSyntaxYaml);
    expect(result.isValid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toBeTruthy();
  });

  it("rejects empty or whitespace-only YAML", () => {
    const result = validateWorkflowYaml("   \n  \n");
    expect(result.isValid).toBe(false);
    expect(result.errors[0].message).toContain("empty");
  });

  it("rejects invalid extractor kind", () => {
    const invalidKindYaml = `
version: 2
metadata:
  name: "Test"
extractor:
  kind: unknown_kind
  url: "https://example.com"
`;
    const result = validateWorkflowYaml(invalidKindYaml);
    expect(result.isValid).toBe(false);
    expect(result.errors.some((e) => e.message.includes("extractor.kind must be one of"))).toBe(true);
  });

  it("rejects non-http/https URL in extractor", () => {
    const invalidUrlYaml = `
version: 2
metadata:
  name: "Test"
extractor:
  kind: greenhouse
  url: "ftp://example.com/jobs"
`;
    const result = validateWorkflowYaml(invalidUrlYaml);
    expect(result.isValid).toBe(false);
    expect(result.errors.some((e) => e.message.includes("http:// or https://"))).toBe(true);
  });

  it("rejects negative retention days", () => {
    const negativeRetentionYaml = `
version: 2
metadata:
  name: "Test"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/test"
retention:
  days: -5
  policy: untracked_only
`;
    const result = validateWorkflowYaml(negativeRetentionYaml);
    expect(result.isValid).toBe(false);
    expect(result.errors.some((e) => e.message.includes("retention.days must be a non-negative number"))).toBe(true);
  });

  it("rejects invalid retention policy", () => {
    const badPolicyYaml = `
version: 2
metadata:
  name: "Test"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/test"
retention:
  days: 30
  policy: purge_everything
`;
    const result = validateWorkflowYaml(badPolicyYaml);
    expect(result.isValid).toBe(false);
    expect(result.errors.some((e) => e.message.includes("retention.policy must be either 'untracked_only' or 'all'"))).toBe(true);
  });

  it("converts WorkflowSpec to CreateSourceInput", () => {
    const spec: WorkflowSpec = {
      version: 2,
      metadata: { name: "Stripe Infra", enabled: true },
      extractor: { kind: "greenhouse", url: "https://boards.greenhouse.io/stripe" },
      schedule: { cadence: "every_12h" },
      filters: { role_types: ["full_time"], include_keywords: ["Rust"] },
      retention: { days: 14, policy: "untracked_only" },
    };

    const input = workflowSpecToCreateInput(spec);
    expect(input.name).toBe("Stripe Infra");
    expect(input.kind).toBe("greenhouse");
    expect(input.identifier).toBe("stripe");
    expect(input.cadence).toBe("every_12h");
    expect(input.retention_days).toBe(14n);
    expect(input.retention_policy).toBe("untracked_only");
    expect((input.filters as any).role_types).toEqual(["full_time"]);
  });

  it("converts Source database record to WorkflowSpec", () => {
    const source: Source = {
      id: "src-1",
      kind: "ashby",
      name: "Ramp",
      identifier: "ramp",
      config: { url: "https://jobs.ashbyhq.com/ramp" },
      enabled: true,
      last_run_at: null,
      last_status: null,
      last_error: null,
      created_at: "2026-10-10T00:00:00Z",
      cadence: "daily",
      cadence_cron: null,
      next_run_at: null,
      last_run_duration_ms: null,
      retention_days: 60n,
      retention_policy: "untracked_only",
      filters: { include_keywords: ["Backend"] },
    };

    const spec = sourceToWorkflowSpec(source);
    expect(spec.version).toBe(3);
    expect(spec.metadata.name).toBe("Ramp");
    expect(spec.extractor.kind).toBe("ashby");
    expect(spec.extractor.url).toBe("https://jobs.ashbyhq.com/ramp");
    expect(spec.schedule?.cadence).toBe("daily");
    expect(spec.retention?.days).toBe(60);
    expect(spec.filters?.include_keywords).toEqual(["Backend"]);

    const serialized = workflowSpecToYaml(spec);
    expect(validateWorkflowYaml(serialized).parsedSpec?.metadata.name).toBe("Ramp");
    expect(serialized).toContain("kind: ashby");
    expect(serialized).toContain("days: 60");
  });

  describe("ATS Slug Canonicalization & Validation", () => {
    it("canonicalizes simple company slugs into full ATS URLs", () => {
      expect(canonicalizeAtsUrl("greenhouse", "stripe")).toBe("https://boards.greenhouse.io/stripe");
      expect(canonicalizeAtsUrl("lever", "figma")).toBe("https://jobs.lever.co/figma");
      expect(canonicalizeAtsUrl("ashby", "ramp")).toBe("https://jobs.ashbyhq.com/ramp");
      expect(canonicalizeAtsUrl("greenhouse", "https://boards.greenhouse.io/stripe")).toBe("https://boards.greenhouse.io/stripe");
      expect(canonicalizeAtsUrl("selector", "https://acme.org/jobs")).toBe("https://acme.org/jobs");
      expect(canonicalizeAtsUrl("selector", "invalid-slug")).toBe("invalid-slug");
    });

    it("accepts valid ATS company slugs directly in extractor.url for greenhouse, lever, and ashby", () => {
      const greenhouseSlugYaml = `
version: 2
metadata:
  name: "Stripe"
extractor:
  kind: greenhouse
  url: stripe
`;
      const ghResult = validateWorkflowYaml(greenhouseSlugYaml);
      expect(ghResult.isValid).toBe(true);
      expect(ghResult.parsedSpec?.extractor.url).toBe("stripe");

      const leverSlugYaml = `
version: 2
metadata:
  name: "Figma"
extractor:
  kind: lever
  url: figma
`;
      const leverResult = validateWorkflowYaml(leverSlugYaml);
      expect(leverResult.isValid).toBe(true);
      expect(leverResult.parsedSpec?.extractor.url).toBe("figma");

      const ashbySlugYaml = `
version: 2
metadata:
  name: "Ramp"
extractor:
  kind: ashby
  url: ramp
`;
      const ashbyResult = validateWorkflowYaml(ashbySlugYaml);
      expect(ashbyResult.isValid).toBe(true);
      expect(ashbyResult.parsedSpec?.extractor.url).toBe("ramp");
    });
  });

  describe("Expressive Filter Policy Engine & SQL LIKE Query Validation", () => {
    it("validates workflow YAML with complex boolean and SQL LIKE filter query", () => {
      const queryYaml = `
version: 2
metadata:
  name: "Custom Query Source"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
filters:
  query: "(title LIKE '%Engineer%' OR title LIKE '%Developer%') AND location LIKE '%Remote%' AND NOT (title LIKE '%Senior%' OR title LIKE '%Staff%')"
  include_keywords: ["Rust"]
`;
      const result = validateWorkflowYaml(queryYaml);
      expect(result.isValid).toBe(true);
      expect(result.errors).toHaveLength(0);
      expect(result.parsedSpec?.filters?.query).toBe(
        "(title LIKE '%Engineer%' OR title LIKE '%Developer%') AND location LIKE '%Remote%' AND NOT (title LIKE '%Senior%' OR title LIKE '%Staff%')"
      );
    });

    it("detects syntax errors in filters.query with diagnostic messages", () => {
      // Unclosed quote
      const unclosedQuoteYaml = `
version: 2
metadata:
  name: "Bad Quote"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
filters:
  query: "title LIKE '%Engineer"
`;
      const quoteResult = validateWorkflowYaml(unclosedQuoteYaml);
      expect(quoteResult.isValid).toBe(false);
      expect(quoteResult.errors.some((e) => e.message.includes("Unclosed string quote"))).toBe(true);

      // Unmatched closing paren
      const unmatchedClosingYaml = `
version: 2
metadata:
  name: "Bad Paren"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
filters:
  query: "title LIKE '%Engineer%')"
`;
      const parenResult = validateWorkflowYaml(unmatchedClosingYaml);
      expect(parenResult.isValid).toBe(false);
      expect(parenResult.errors.some((e) => e.message.includes("Unmatched closing parenthesis"))).toBe(true);

      // Unclosed opening paren
      const unclosedOpeningYaml = `
version: 2
metadata:
  name: "Bad Paren 2"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
filters:
  query: "(title LIKE '%Engineer%'"
`;
      const unclosedResult = validateWorkflowYaml(unclosedOpeningYaml);
      expect(unclosedResult.isValid).toBe(false);
      expect(unclosedResult.errors.some((e) => e.message.includes("Unclosed opening parenthesis"))).toBe(true);
    });

    it("round-trips filters.query and filters.policy through spec conversion and YAML", () => {
      const spec: WorkflowSpec = {
        version: 2,
        metadata: { name: "Policy Test", enabled: true },
        extractor: { kind: "lever", url: "https://jobs.lever.co/figma" },
        filters: {
          query: "title LIKE '%Intern%' AND location LIKE '%Remote%'",
          policy: {
            all: [
              { field: "title", op: "like", value: "%Intern%" },
              { field: "location", op: "like", value: "%Remote%" },
            ],
          },
        },
      };

      const serialized = workflowSpecToYaml(spec);

      const parsed = validateWorkflowYaml(serialized);
      expect(parsed.isValid).toBe(true);
      expect(parsed.parsedSpec?.filters?.query).toBe("title LIKE '%Intern%' AND location LIKE '%Remote%'");
      expect(parsed.parsedSpec?.filters?.policy).toEqual({
        all: [
          { field: "title", op: "like", value: "%Intern%" },
          { field: "location", op: "like", value: "%Remote%" },
        ],
      });

      const createInput = workflowSpecToCreateInput(spec);
      expect((createInput.filters as any).query).toBe("title LIKE '%Intern%' AND location LIKE '%Remote%'");
      expect((createInput.filters as any).policy).toBeDefined();

      const sourceRecord: Source = {
        id: "src-test",
        kind: "lever",
        name: "Policy Test",
        identifier: "figma",
        config: { url: "https://jobs.lever.co/figma" },
        enabled: true,
        last_run_at: null,
        last_status: null,
        last_error: null,
        created_at: "2026-10-10T00:00:00Z",
        cadence: "daily",
        cadence_cron: null,
        next_run_at: null,
        last_run_duration_ms: null,
        retention_days: 30n,
        retention_policy: "untracked_only",
        filters: {
          query: "title LIKE '%Intern%'",
          policy: { all: [{ field: "title", op: "like", value: "%Intern%" }] },
        },
      };

      const convertedSpec = sourceToWorkflowSpec(sourceRecord);
      expect(convertedSpec.filters?.query).toBe("title LIKE '%Intern%'");
      expect(convertedSpec.filters?.policy).toBeDefined();
    });
  });
});
