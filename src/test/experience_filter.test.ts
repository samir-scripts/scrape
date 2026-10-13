import { describe, it, expect } from "vitest";
import {
  CAREER_LEVEL_PRESETS,
  NEW_GRAD_OPTIONS,
} from "../components/filters/ExperienceFilter";
import { matchesJob } from "../lib/filter";
import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";

const createMockJob = (overrides: Partial<Job> = {}): Job => ({
  id: "job-1",
  dedupe_key: "gh:test:1",
  source_id: "src-1",
  external_id: "1",
  title: "Software Engineer",
  company: "Acme",
  role_type: "full_time",
  location: "Remote",
  url: "https://example.com/1",
  date_posted: null,
  date_added: "2026-10-09T00:00:00Z",
  last_seen_at: "2026-10-09T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-09T00:00:00Z",
  notes: null,
  favorited: false,
  favorited_at: null,
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
  ...overrides,
});

describe("Consolidated Experience & Career Level Filter", () => {
  it("provides comprehensive career level presets covering all experience tiers", () => {
    const labels = CAREER_LEVEL_PRESETS.map((p) => p.label);
    expect(labels).toContain("ALL");
    expect(labels).toContain("NEW GRAD (0-1)");
    expect(labels).toContain("MID (1-3)");
    expect(labels).toContain("SENIOR (3-5)");
    expect(labels).toContain("STAFF+ (5+)");

    const newGradPreset = CAREER_LEVEL_PRESETS.find((p) => p.label === "NEW GRAD (0-1)")!;
    expect(newGradPreset.min).toBe(0);
    expect(newGradPreset.max).toBe(1);

    const seniorPreset = CAREER_LEVEL_PRESETS.find((p) => p.label === "SENIOR (3-5)")!;
    expect(seniorPreset.min).toBe(3);
    expect(seniorPreset.max).toBe(5);

    const staffPreset = CAREER_LEVEL_PRESETS.find((p) => p.label === "STAFF+ (5+)")!;
    expect(staffPreset.min).toBe(5);
    expect(staffPreset.max).toBeNull();
  });

  it("provides clean New Grad status qualification options", () => {
    const ids = NEW_GRAD_OPTIONS.map((o) => o.id);
    expect(ids).toEqual(["any", "only", "exclude"]);
  });

  it("filters accurately when applying New Grad preset (0-1 YOE)", () => {
    const entryJob = createMockJob({ experience_min: 0, experience_max: 1, is_new_grad: "yes" });
    const midJob = createMockJob({ experience_min: 2, experience_max: 4, is_new_grad: "no" });
    const seniorJob = createMockJob({ experience_min: 5, experience_max: null, is_new_grad: "no" });

    const newGradPreset = CAREER_LEVEL_PRESETS.find((p) => p.label === "NEW GRAD (0-1)")!;
    const filter: Partial<JobFilter> = {
      experience_min: newGradPreset.min,
      experience_max: newGradPreset.max,
      new_grad: "any",
    };

    expect(matchesJob(entryJob, filter)).toBe(true);
    expect(matchesJob(midJob, filter)).toBe(false);
    expect(matchesJob(seniorJob, filter)).toBe(false);
  });

  it("filters accurately when requiring new_grad = only or exclude within the same criteria", () => {
    const newGradWithYoe = createMockJob({ experience_min: 0, experience_max: 1, is_new_grad: "yes" });
    const juniorNotGrad = createMockJob({ experience_min: 0, experience_max: 1, is_new_grad: "no" });

    // 0-1 YOE with NEW GRAD ONLY
    const newGradOnlyFilter: Partial<JobFilter> = {
      experience_min: 0,
      experience_max: 1,
      new_grad: "only",
    };
    expect(matchesJob(newGradWithYoe, newGradOnlyFilter)).toBe(true);
    expect(matchesJob(juniorNotGrad, newGradOnlyFilter)).toBe(false);

    // 0-1 YOE with EXCLUDE NEW GRAD
    const excludeGradFilter: Partial<JobFilter> = {
      experience_min: 0,
      experience_max: 1,
      new_grad: "exclude",
    };
    expect(matchesJob(newGradWithYoe, excludeGradFilter)).toBe(false);
    expect(matchesJob(juniorNotGrad, excludeGradFilter)).toBe(true);
  });
});
