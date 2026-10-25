import { afterEach, beforeEach, describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  validateWorkflowYaml, workflowSpecToCreateInput, workflowSpecToYaml,
  SAMPLE_WORKFLOW_TEMPLATES, WORKFLOW_TEMPLATE_CATALOG,
} from "../lib/yamlWorkflow";
import { api, __resetDevData, __seedDevJobs, generateScrapedJobsForSource } from "../lib/api";

const expectedIds = [
  "software-engineering-last-12-hours",
  "cherry-hill-fast-food-last-month",
  "remote-usd-over-60000",
];

beforeEach(() => __resetDevData());
afterEach(() => __resetDevData());

describe("targeted workflow catalog", () => {
  it("projects exactly the three current executable profiles to generated YAML and picker specs", () => {
    expect(WORKFLOW_TEMPLATE_CATALOG.version).toBe(3);
    expect(SAMPLE_WORKFLOW_TEMPLATES.map(template => template.id)).toEqual(expectedIds);
    for (const template of SAMPLE_WORKFLOW_TEMPLATES) {
      const yaml = readFileSync(resolve(__dirname, `../../workflows/${template.id}.yaml`), "utf8");
      const parsed = validateWorkflowYaml(yaml);
      expect(parsed.errors, template.id).toEqual([]);
      expect(parsed.parsedSpec, template.id).toEqual(template.spec);
      expect(validateWorkflowYaml(workflowSpecToYaml(template.spec)).parsedSpec).toEqual(template.spec);
      const input = workflowSpecToCreateInput(template.spec);
      expect((input.config as Record<string, unknown>).__workflow_spec).toEqual(template.spec);
      expect(input.filters).toEqual(template.spec.filters);
      expect(template.spec.max_jobs_per_run).toBe(20);
      expect(template.spec.schedule?.cadence).toBe("manual");
    }
  });

  it("keeps date, place, role, evidence, and salary requirements explicit in Rust-owned profiles", () => {
    const engineering = SAMPLE_WORKFLOW_TEMPLATES[0].spec.filters!.matching!;
    expect(engineering.posting_date).toMatchObject({ preset: "past_12h", timezone: "America/New_York", include_unknown: false });
    expect(engineering.target_roles).toContain("Firmware Engineer");
    expect(engineering.target_roles).not.toContain("Engineer");
    expect(engineering.countries).toEqual([]);
    expect(engineering.cities).toEqual([]);
    expect(SAMPLE_WORKFLOW_TEMPLATES[0].spec.sources.some(source => source.extractor.kind === "search" && source.extractor.url === "adzuna" && !source.enabled)).toBe(true);

    const fastFood = SAMPLE_WORKFLOW_TEMPLATES[1].spec.filters!.matching!;
    expect(fastFood.posting_date).toMatchObject({ preset: "past_month", timezone: "America/New_York", include_unknown: false });
    expect(fastFood.cities).toEqual(["Cherry Hill"]);
    expect(fastFood.regions).toEqual(["US-NJ"]);
    expect(fastFood.countries).toEqual(["US"]);
    expect(fastFood.target_roles).toContain("Crew Member");
    expect(fastFood.keywords).toMatchObject({ any: expect.arrayContaining(["fast food", "McDonald's", "Taco Bell"]), all: [], exclude: [], fields: ["title", "company", "description"] });

    const remote = SAMPLE_WORKFLOW_TEMPLATES[2].spec.filters!.matching!;
    expect(remote.workplaces).toEqual(["remote"]);
    expect(remote.compensation).toMatchObject({ min_minor: 6000001, currency: "USD", interval: "year", include_unknown: false, match_mode: "advertised_amount" });
    expect(remote.posting_date).toBeNull();
    expect(remote.cities).toEqual([]);
  });

  it("preserves current built-in IDs and saved jobs on an empty replacement selection", async () => {
    const before = await api.listSources();
    const job = generateScrapedJobsForSource(before[0])[0];
    __seedDevJobs([job]);
    const preview = await api.previewTestWorkflowReplacement();
    expect(preview.candidates).toHaveLength(3);
    expect(preview.candidates.every(candidate => candidate.selected_by_default)).toBe(false);

    const report = await api.replaceTestWorkflows([]);
    const after = await api.listSources();
    expect(report.created).toEqual([]);
    expect(report.removed_ids).toEqual([]);
    expect(after.map(source => source.id)).toEqual(before.map(source => source.id));
    expect((await api.listJobs()).items.map(saved => saved.id)).toEqual([job.id]);
  });
});
