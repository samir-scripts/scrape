import { describe, it, expect } from "vitest";
import { readFileSync } from "fs";
import { resolve } from "path";
import {
  validateWorkflowYaml,
  workflowSpecToCreateInput,
  SAMPLE_SWE_INTERNSHIP_YAML,
  SAMPLE_SWE_INTERNSHIP_SPEC,
} from "../lib/yamlWorkflow";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";

describe("Sample Workflow Suite: Software Engineering Internships", () => {
  it("validates the shipped workflows/software-engineering-internships.yaml file", () => {
    const yamlPath = resolve(__dirname, "../../workflows/software-engineering-internships.yaml");
    const yamlContent = readFileSync(yamlPath, "utf-8");

    const result = validateWorkflowYaml(yamlContent);
    expect(result.isValid).toBe(true);
    expect(result.errors).toHaveLength(0);
    expect(result.parsedSpec).toBeDefined();

    const spec = result.parsedSpec!;
    expect(spec.version).toBe(2);
    expect(spec.metadata.name).toBe("Cloudflare — Software Engineering Internships");
    expect(spec.metadata.company).toBe("Cloudflare");
    expect(spec.metadata.enabled).toBe(true);

    expect(spec.sources[0].extractor.kind).toBe("greenhouse");
    expect(spec.sources[0].extractor.url).toBe("https://boards.greenhouse.io/cloudflare");

    expect(spec.schedule?.cadence).toBe("daily");
    expect(spec.schedule?.cron).toBe("0 0 9 * * *");

    expect(spec.filters?.role_types).toEqual(["internship"]);
    expect(spec.filters?.include_keywords).toEqual(["Software", "Engineer", "Intern"]);
    expect(spec.filters?.exclude_keywords).toEqual(["MBA", "Sales", "Marketing"]);
    expect(spec.filters?.locations).toContain("Remote");
    expect(spec.filters?.locations).toContain("San Francisco, CA");

    expect(spec.retention?.days).toBe(45);
    expect(spec.retention?.policy).toBe("untracked_only");
  });

  it("converts sample workflow spec to valid CreateSourceInput", () => {
    const createInput = workflowSpecToCreateInput(SAMPLE_SWE_INTERNSHIP_SPEC);

    expect(createInput.kind).toBe("greenhouse");
    expect(createInput.name).toBe("Cloudflare — Software Engineering Internships");
    expect(createInput.identifier).toBe("cloudflare");
    expect(createInput.enabled).toBe(true);
    expect(createInput.cadence).toBe("daily");
    expect(createInput.cadence_cron).toBe("0 0 9 * * *");
    expect(createInput.retention_days).toBe(BigInt(45));
    expect(createInput.retention_policy).toBe("untracked_only");

    const filters = createInput.filters as Record<string, unknown>;
    expect(filters.role_types).toEqual(["internship"]);
    expect(filters.include_keywords).toEqual(["Software", "Engineer", "Intern"]);
  });

  it("exports valid SAMPLE_SWE_INTERNSHIP_YAML that passes validation", () => {
    const result = validateWorkflowYaml(SAMPLE_SWE_INTERNSHIP_YAML);
    expect(result.isValid).toBe(true);
    expect(result.parsedSpec?.metadata.name).toBe("Cloudflare — Software Engineering Internships");
  });

  it("verifies SAMPLE_SWE_SOURCE constant structure", () => {
    expect(SAMPLE_SWE_SOURCE.name).toBe("Cloudflare — Software Engineering Internships");
    expect(SAMPLE_SWE_SOURCE.identifier).toBe("cloudflare");
    expect(SAMPLE_SWE_SOURCE.kind).toBe("greenhouse");
    expect(SAMPLE_SWE_SOURCE.cadence).toBe("daily");
    expect(SAMPLE_SWE_SOURCE.retention_days).toBe(BigInt(45));
    expect(SAMPLE_SWE_SOURCE.filters).toHaveProperty("role_types");
  });

  it("provides sample workflow through api.listSources and api.seedSampleWorkflow", async () => {
    const sources = await api.listSources();
    expect(sources.length).toBeGreaterThan(0);

    const sample = sources.find((s) => s.identifier === "cloudflare");
    expect(sample).toBeDefined();
    expect(sample?.name).toBe("Cloudflare — Software Engineering Internships");

    // Re-seeding when already present returns existing
    const seeded = await api.seedSampleWorkflow();
    expect(seeded.identifier).toBe("cloudflare");

    // Single source run trigger executes
    const run = await api.runSourceNow(sample!.id);
    expect(run.status).toBe("success");
    expect(run.jobs_found).toBe(BigInt(8));
  });
});
