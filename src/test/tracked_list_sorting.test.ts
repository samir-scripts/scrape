import { describe, it, expect } from "vitest";
import { compareTrackedJobs } from "../components/views/tracked/TrackedList";
import type { Job } from "../bindings/Job";

const createMockJob = (overrides: Partial<Job> = {}): Job => ({
  id: "job-default",
  dedupe_key: "key-1",
  source_id: "src-1",
  external_id: "ext-1",
  title: "Software Engineer",
  company: "Acme",
  role_type: "full_time",
  location: "Remote",
  url: "https://example.com",
  date_posted: "2026-10-01T00:00:00Z",
  date_added: "2026-10-05T12:00:00Z",
  last_seen_at: "2026-10-05T12:00:00Z",
  status: "applied",
  status_changed_at: "2026-10-05T12:00:00Z",
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
  experience_min: null,
  experience_max: null,
  experience_raw: null,
  is_new_grad: "unknown",
  experience_evidence: null,
  description_text: null,
  enrichment_version: 1,
  locations: [],
  ...overrides,
});

describe("Tracked List Sorting - compareTrackedJobs", () => {
  it("sorts by company ascending and descending", () => {
    const jobA = createMockJob({ id: "1", company: "Apple" });
    const jobB = createMockJob({ id: "2", company: "Zebra" });

    expect(compareTrackedJobs(jobA, jobB, "company", true)).toBeLessThan(0);
    expect(compareTrackedJobs(jobA, jobB, "company", false)).toBeGreaterThan(0);
    expect(compareTrackedJobs(jobA, jobA, "company", true)).toBe(0);
  });

  it("sorts by job title alphabetically", () => {
    const jobA = createMockJob({ id: "1", title: "Backend Engineer" });
    const jobB = createMockJob({ id: "2", title: "Frontend Engineer" });

    expect(compareTrackedJobs(jobA, jobB, "title", true)).toBeLessThan(0);
    expect(compareTrackedJobs(jobA, jobB, "title", false)).toBeGreaterThan(0);
  });

  it("sorts by status stage", () => {
    const jobApplied = createMockJob({ id: "1", status: "applied" });
    const jobInterview = createMockJob({ id: "2", status: "interviewing" });

    expect(compareTrackedJobs(jobApplied, jobInterview, "status", true)).toBeLessThan(0);
    expect(compareTrackedJobs(jobApplied, jobInterview, "status", false)).toBeGreaterThan(0);
  });

  it("sorts by salary with nulls handled gracefully", () => {
    const jobNull = createMockJob({ id: "1", salary_min: null, salary_max: null });
    const jobLow = createMockJob({ id: "2", salary_min: 120000, salary_max: 150000 });
    const jobHigh = createMockJob({ id: "3", salary_min: 200000, salary_max: 250000 });
    const jobMaxOnly = createMockJob({ id: "4", salary_min: null, salary_max: 180000 });

    const jobs = [jobHigh, jobNull, jobLow, jobMaxOnly];

    // Ascending: null (-1) -> 120k -> 180k -> 200k
    const ascSorted = [...jobs].sort((a, b) => compareTrackedJobs(a, b, "salary", true));
    expect(ascSorted.map((j) => j.id)).toEqual(["1", "2", "4", "3"]);

    // Descending: 200k -> 180k -> 120k -> null (-1)
    const descSorted = [...jobs].sort((a, b) => compareTrackedJobs(a, b, "salary", false));
    expect(descSorted.map((j) => j.id)).toEqual(["3", "4", "2", "1"]);
  });

  it("sorts by date_added chronologically", () => {
    const jobOld = createMockJob({ id: "1", date_added: "2026-10-01T10:00:00Z" });
    const jobNew = createMockJob({ id: "2", date_added: "2026-10-09T18:00:00Z" });

    expect(compareTrackedJobs(jobOld, jobNew, "date_added", true)).toBeLessThan(0);
    expect(compareTrackedJobs(jobOld, jobNew, "date_added", false)).toBeGreaterThan(0);
  });
});
