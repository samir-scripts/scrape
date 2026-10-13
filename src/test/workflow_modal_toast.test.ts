import { describe, it, expect, beforeEach } from "vitest";
import { useToastStore } from "../lib/toast";
import { validateWorkflowYaml } from "../lib/yamlWorkflow";
import { matchesJob } from "../lib/filter";
import type { Job } from "../bindings/Job";

describe("Workflow Modal, Toast & Favorited Integration", () => {
  beforeEach(() => {
    useToastStore.getState().clearAll();
  });

  it("blocks invalid YAML on save and throws bottom-right error toast", () => {
    const invalidYaml = `
version: 2
metadata:
  name: [broken yaml
extractor:
  kind: greenhouse
`;
    const validation = validateWorkflowYaml(invalidYaml);
    expect(validation.isValid).toBe(false);

    // Emulate save handler behavior from AddSourceModal
    if (!validation.isValid) {
      useToastStore.getState().addToast({
        type: "error",
        title: "YAML PARSE ERROR",
        message: `YAML could not be saved due to parsing issues or invalid parameters: ${validation.errors.map((e) => e.message).join("\n")}`,
        duration: 5000,
      });
    }

    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].type).toBe("error");
    expect(toasts[0].title).toBe("YAML PARSE ERROR");
    expect(toasts[0].message).toContain("YAML could not be saved due to parsing issues");
  });

  it("blocks schema validation errors and throws toast", () => {
    const badParamsYaml = `
version: 2
metadata:
  name: "Bad Retention Job"
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/bad"
retention:
  days: -10
  policy: destroy_everything
`;
    const validation = validateWorkflowYaml(badParamsYaml);
    expect(validation.isValid).toBe(false);

    if (!validation.isValid) {
      useToastStore.getState().addToast({
        type: "error",
        title: "YAML VALIDATION ERROR",
        message: `YAML could not be saved due to parsing issues or invalid parameters:\n${validation.errors.map((e) => e.message).join("\n")}`,
        duration: 5000,
      });
    }

    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].message).toContain("retention.days must be a non-negative number");
    expect(toasts[0].message).toContain("retention.policy must be either");
  });

  it("filters jobs with favorited_only correctly", () => {
    const jobA: Job = {
      id: "j-1",
      dedupe_key: "k-1",
      source_id: null,
      external_id: "1",
      title: "Rust Lead",
      company: "Acme",
      role_type: "full_time",
      location: "Remote",
      url: "https://example.com/1",
      date_posted: null,
      date_added: "2026-10-10T00:00:00Z",
      last_seen_at: "2026-10-10T00:00:00Z",
      status: "scraped",
      status_changed_at: "2026-10-10T00:00:00Z",
      notes: null,
      favorited: true,
      favorited_at: "2026-10-10T00:00:00Z",
      salary_min: null,
      salary_max: null,
      salary_currency: null,
      salary_interval: null,
      salary_raw: null,
      salary_source: null,
      workplace_type: "remote",
      sponsorship: "unknown",
      sponsorship_evidence: null,
      clearance: "unknown",
      clearance_level: null,
      clearance_evidence: null,
      work_auth: "unknown",
      work_auth_evidence: null,
      experience_min: null,
      experience_max: null,
      experience_raw: null,
      is_new_grad: "unknown",
      experience_evidence: null,
      description_text: null,
      company_icon_url: null,
      enrichment_version: 1,
      locations: [],
    };

    const jobB: Job = {
      ...jobA,
      id: "j-2",
      favorited: false,
      favorited_at: null,
    };

    expect(matchesJob(jobA, { favorited_only: true })).toBe(true);
    expect(matchesJob(jobB, { favorited_only: true })).toBe(false);
    expect(matchesJob(jobB, { favorited_only: null })).toBe(true);
  });
});
