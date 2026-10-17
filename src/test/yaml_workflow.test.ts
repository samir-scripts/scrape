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
  it("preserves mixed CSS/XPath locators and field transforms in YAML", () => {
    const config = {
      item: "//article[contains(@class, 'job-card')]",
      title: "xpath:.//h2/text()",
      title_regex: "Role: (?P<value>.+)",
      link: "a@data-url",
      id: ".",
      id_attribute: "data-job-id",
      date: "time",
      date_attribute: "datetime",
      description: "p",
      description_attribute: "text",
    };
    const spec: WorkflowSpec = {
      version: 5,
      metadata: { name: "Locator fixture" },
      sources: [{
        id: "fixture", label: "Fixture", enabled: true,
        extractor: { kind: "selector", url: "https://fixture.test/careers", source_config: config },
      }],
      schedule: { cadence: "manual" },
    };
    const result = validateWorkflowYaml(workflowSpecToYaml(spec));
    expect(result.errors).toEqual([]);
    expect(result.parsedSpec?.sources[0].extractor.source_config).toEqual(config);
    // Rust performs locator validation; the YAML serializer must preserve it verbatim.
    expect(workflowSpecToCreateInput(result.parsedSpec!).config).toMatchObject(config);
  });

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
    expect(spec.sources[0].extractor.kind).toBe("greenhouse");
    expect(spec.sources[0].extractor.url).toBe("https://boards.greenhouse.io/stripe");
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
      version: 5,
      metadata: { name: "Stripe Infra", enabled: true },
      sources: [
        {
          id: "source-1",
          label: "Stripe Infra",
          enabled: true,
          extractor: { kind: "greenhouse", url: "https://boards.greenhouse.io/stripe" },
        },
      ],
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
      needs_review: false, script_trusted: false,
    };

    const spec = sourceToWorkflowSpec(source);
    expect(spec.version).toBe(6);
    expect(spec.metadata.name).toBe("Ramp");
    expect(spec.sources[0].extractor.kind).toBe("ashby");
    expect(spec.sources[0].extractor.url).toBe("https://jobs.ashbyhq.com/ramp");
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
      expect(ghResult.parsedSpec?.sources[0].extractor.url).toBe("stripe");

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
      expect(leverResult.parsedSpec?.sources[0].extractor.url).toBe("figma");

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
      expect(ashbyResult.parsedSpec?.sources[0].extractor.url).toBe("ramp");
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
        version: 5,
        metadata: { name: "Policy Test", enabled: true },
        sources: [
          {
            id: "source-1",
            label: "Policy Test",
            enabled: true,
            extractor: { kind: "lever", url: "https://jobs.lever.co/figma" },
          },
        ],
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
        needs_review: false, script_trusted: false,
      };

      const convertedSpec = sourceToWorkflowSpec(sourceRecord);
      expect(convertedSpec.filters?.query).toBe("title LIKE '%Intern%'");
      expect(convertedSpec.filters?.policy).toBeDefined();
    });
  });

  describe("Multi-source Workflow Validation & Ownership (Section 5)", () => {
    it("validates a multi-source v5 YAML workflow with multiple distinct sources", () => {
      const multiSourceYaml = `
version: 5
metadata:
  name: "Multi-Source SWE Workflow"
  company: "Various"
sources:
  - id: "src-cf"
    label: "Cloudflare"
    enabled: true
    employer_hint: "Cloudflare"
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/cloudflare"
  - id: "src-figma"
    label: "Figma"
    enabled: true
    employer_hint: "Figma"
    extractor:
      kind: lever
      url: "https://jobs.lever.co/figma"
  - id: "src-ramp"
    label: "Ramp"
    enabled: false
    employer_hint: "Ramp"
    extractor:
      kind: ashby
      url: "https://jobs.ashbyhq.com/ramp"
filters:
  include_keywords: ["Software", "Engineer"]
schedule:
  cadence: daily
`;
      const result = validateWorkflowYaml(multiSourceYaml);
      expect(result.isValid).toBe(true);
      expect(result.errors).toHaveLength(0);
      expect(result.parsedSpec?.sources).toHaveLength(3);
      expect(result.parsedSpec?.sources[0].id).toBe("src-cf");
      expect(result.parsedSpec?.sources[0].enabled).toBe(true);
      expect(result.parsedSpec?.sources[1].id).toBe("src-figma");
      expect(result.parsedSpec?.sources[2].id).toBe("src-ramp");
      expect(result.parsedSpec?.sources[2].enabled).toBe(false);
      expect(result.parsedSpec?.filters?.include_keywords).toEqual(["Software", "Engineer"]);
    });

    it("rejects duplicate normalized endpoints across different sources in the same workflow", () => {
      const duplicateEndpointYaml = `
version: 5
metadata:
  name: "Duplicate Endpoint Workflow"
sources:
  - id: "source-1"
    label: "Stripe Full URL"
    enabled: true
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/stripe"
  - id: "source-2"
    label: "Stripe Slug"
    enabled: true
    extractor:
      kind: greenhouse
      url: "stripe"
`;
      const result = validateWorkflowYaml(duplicateEndpointYaml);
      expect(result.isValid).toBe(false);
      expect(result.errors.some((e) => e.message.includes("Duplicate source endpoint"))).toBe(true);
    });

    it("rejects workflows with no enabled sources", () => {
      const allDisabledYaml = `
version: 5
metadata:
  name: "All Disabled"
sources:
  - id: "src-1"
    label: "Source 1"
    enabled: false
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/stripe"
`;
      const result = validateWorkflowYaml(allDisabledYaml);
      expect(result.isValid).toBe(false);
      expect(result.errors.some((e) => e.message.includes("at least one source must be enabled"))).toBe(true);
    });

    it("rejects workflows with empty sources list", () => {
      const emptySourcesYaml = `
version: 5
metadata:
  name: "Empty Sources"
sources: []
`;
      const result = validateWorkflowYaml(emptySourcesYaml);
      expect(result.isValid).toBe(false);
      expect(result.errors.some((e) => e.message.includes("must contain at least one source"))).toBe(true);
    });

    it("rejects duplicate source IDs in the same workflow", () => {
      const duplicateIdsYaml = `
version: 5
metadata:
  name: "Duplicate IDs"
sources:
  - id: "dup-id"
    label: "Source 1"
    enabled: true
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/stripe"
  - id: "dup-id"
    label: "Source 2"
    enabled: true
    extractor:
      kind: lever
      url: "https://jobs.lever.co/figma"
`;
      const result = validateWorkflowYaml(duplicateIdsYaml);
      expect(result.isValid).toBe(false);
      expect(result.errors.some((e) => e.message.includes("duplicate source ID 'dup-id'"))).toBe(true);
    });

    it("rejects blank source ID and blank source label", () => {
      const blankIdYaml = `
version: 5
metadata:
  name: "Blank ID"
sources:
  - id: "   "
    label: "   "
    enabled: true
    extractor:
      kind: greenhouse
      url: "https://boards.greenhouse.io/stripe"
`;
      const result = validateWorkflowYaml(blankIdYaml);
      expect(result.isValid).toBe(false);
      expect(result.errors.some((e) => e.message.includes(".id: must not be blank"))).toBe(true);
      expect(result.errors.some((e) => e.message.includes(".label: must not be blank"))).toBe(true);
    });
  });
});
