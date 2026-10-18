import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  validateWorkflowYaml, workflowSpecToCreateInput, workflowSpecToYaml,
  SAMPLE_WORKFLOW_TEMPLATES, WORKFLOW_TEMPLATE_CATALOG,
} from "../lib/yamlWorkflow";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";

describe("Rust-owned template projections", () => {
  it("imports every generated YAML as the same spec used by the picker and seed input", () => {
    expect(WORKFLOW_TEMPLATE_CATALOG.version).toBe(1);
    expect(SAMPLE_WORKFLOW_TEMPLATES.filter(template => template.seed)).toHaveLength(6);
    for (const template of SAMPLE_WORKFLOW_TEMPLATES) {
      const yaml = readFileSync(resolve(__dirname, `../../workflows/${template.id}.yaml`), "utf8");
      const parsed = validateWorkflowYaml(yaml);
      expect(parsed.errors, template.id).toEqual([]);
      expect(parsed.parsedSpec, template.id).toEqual(template.spec);
      expect(validateWorkflowYaml(workflowSpecToYaml(template.spec)).parsedSpec).toEqual(template.spec);
      const input = workflowSpecToCreateInput(template.spec);
      expect((input.config as Record<string, unknown>).__workflow_spec).toEqual(template.spec);
      expect(input.filters).toEqual(template.spec.filters);
    }
  });

  it("keeps strict examples separate and new ATS templates free of unknown-evidence restrictions", () => {
    for (const template of SAMPLE_WORKFLOW_TEMPLATES.filter(template => template.readiness === "ready")) {
      const filters = template.spec.filters!;
      expect(filters.role_types).toEqual([]);
      const matching = filters.matching!;
      expect(matching.role_types).toEqual([]);
      expect(matching.workplaces).toEqual([]);
      expect(matching.countries).toEqual([]);
      expect(matching.cities).toEqual([]);
      expect(matching.keywords?.fields).toEqual(["title"]);
      expect(matching.keywords?.any.length).toBeGreaterThan(0);
    }
    const restrictive = SAMPLE_WORKFLOW_TEMPLATES.filter(template => template.readiness === "restrictive_example");
    expect(restrictive).toHaveLength(5);
    expect(restrictive.every(template => !template.seed && template.name.includes("Restrictive"))).toBe(true);
    const scripts = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "streaming-cloud-script")!;
    expect(scripts.spec.metadata.enabled).toBe(false);
    for (const source of scripts.spec.sources) {
      expect(source.enabled).toBe(false);
      expect(source.extractor.script?.path).toBeUndefined();
      expect(source.extractor.script?.interpreter).toBeUndefined();
      expect(source.extractor.script?.code).toContain("version: 2");
    }
  });

  it("keeps the existing labelled demonstration workflow stable across reseeding", async () => {
    const sources = await api.listSources();
    const sample = sources.find(source => source.identifier === "cloudflare")!;
    expect(sample.id).toBe(SAMPLE_SWE_SOURCE.id);
    const before = structuredClone(sample);
    const seeded = await api.seedSampleWorkflow();
    expect(seeded).toEqual(before);
    const run = await api.runSourceNow(sample.id);
    expect(run.status).toBe("success");
    expect(run.jobs_found).toBe(BigInt(8));
  });
});
