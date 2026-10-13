import { describe, it, expect } from "vitest";
import type { Job } from "../bindings/Job";
import {
  matchesJob,
  formatSalary,
  countActiveFilters,
} from "../lib/filter";
import {
  parseHourlyInput,
  parseSalaryInput,
} from "../components/filters/SalaryRangeFilter";

const createMockJob = (overrides: Partial<Job> = {}): Job => ({
  id: "job-1",
  dedupe_key: "test:job:1",
  source_id: "src-1",
  external_id: "1",
  title: "Frontend Developer",
  company: "Starlight Corp",
  role_type: "full_time",
  location: "Austin, TX",
  url: "https://example.com/jobs/1",
  date_posted: "2026-10-01T00:00:00Z",
  date_added: "2026-10-09T00:00:00Z",
  last_seen_at: "2026-10-09T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-09T00:00:00Z",
  notes: "Follow up next Monday",
  favorited: false,
  favorited_at: null,
  salary_min: 104000,
  salary_max: 145600,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$104,000 - $145,600",
  salary_source: "structured",
  workplace_type: "remote",
  sponsorship: "yes",
  sponsorship_evidence: "Sponsorship offered.",
  clearance: "no",
  clearance_level: null,
  clearance_evidence: "No clearance.",
  work_auth: "open",
  work_auth_evidence: "Sponsorship offered.",
  experience_min: 3,
  experience_max: 5,
  experience_raw: "3-5 years",
  is_new_grad: "no",
  experience_evidence: "3+ years React.",
  description_text: "Seeking an engineer proficient in TypeScript, React, and Tailwind CSS.",
  company_icon_url: null,
  enrichment_version: 1,
  locations: [],
  ...overrides,
});

describe("Contains / Keyword Filter Logic", () => {
  it("matches job by title substring", () => {
    const job = createMockJob({ title: "Staff Rust Engineer" });
    expect(matchesJob(job, { search: "rust" })).toBe(true);
    expect(matchesJob(job, { search: "python" })).toBe(false);
  });

  it("matches job by company substring", () => {
    const job = createMockJob({ company: "OpenAI Labs" });
    expect(matchesJob(job, { search: "openai" })).toBe(true);
    expect(matchesJob(job, { search: "google" })).toBe(false);
  });

  it("matches job by location substring", () => {
    const job = createMockJob({ location: "Seattle, WA" });
    expect(matchesJob(job, { search: "seattle" })).toBe(true);
  });

  it("matches job by description_text keywords", () => {
    const job = createMockJob({
      description_text: "Must have deep experience with GraphQL, Docker, and Kubernetes.",
    });
    expect(matchesJob(job, { search: "graphql" })).toBe(true);
    expect(matchesJob(job, { search: "kubernetes" })).toBe(true);
    expect(matchesJob(job, { search: "cobol" })).toBe(false);
  });

  it("matches job by user notes", () => {
    const job = createMockJob({ notes: "Spoke with recruiter Sarah on LinkedIn" });
    expect(matchesJob(job, { search: "recruiter sarah" })).toBe(true);
  });

  it("handles whitespace and empty search strings", () => {
    const job = createMockJob();
    expect(matchesJob(job, { search: "" })).toBe(true);
    expect(matchesJob(job, { search: "   " })).toBe(true);
  });

  it("increments countActiveFilters when search is non-empty", () => {
    expect(countActiveFilters({ search: "react" })).toBe(1);
    expect(countActiveFilters({ search: "   " })).toBe(0);
    expect(countActiveFilters({ search: null })).toBe(0);
  });
});

describe("native compensation units", () => {
  const hourly = (min: number | null, max: number | null, currency = "USD"): Job => createMockJob({
    salary_interval: "hour", compensation: [{ min_minor: min, max_minor: max, currency, interval: "hour", raw: "Hourly fixture", source: "structured", derived: false, location: null }],
  });
  it("preserves cents and never guesses units from magnitude", () => {
    expect(parseHourlyInput("$45.25/hr")).toBe(45.25);
    expect(parseHourlyInput("45.255")).toBeNull();
    expect(parseSalaryInput("150")).toBe(150);
    expect(parseSalaryInput("150k")).toBe(150000);
    expect(parseSalaryInput("50garbage")).toBeNull();
    expect(formatSalary(hourly(4525, 5575))).toBe("$45.25 - $55.75/HR");
  });
  it("uses inclusive native overlap and one-sided bounds", () => {
    const filter = { salary_min: 50, salary_max: 60, salary_interval: "hour" as const, salary_currency: "USD" };
    expect(matchesJob(hourly(4500, 5500), filter)).toBe(true);
    expect(matchesJob(hourly(null, 5000), filter)).toBe(true);
    expect(matchesJob(hourly(6000, null), filter)).toBe(true);
    expect(matchesJob(hourly(4000, 4999), filter)).toBe(false);
    expect(matchesJob(hourly(4500, 5500, "EUR"), filter)).toBe(false);
    expect(matchesJob(createMockJob(), filter)).toBe(false);
    expect(matchesJob(createMockJob({ salary_interval: "hour", compensation: [] }), filter)).toBe(false);
  });
  it("keeps salary interval as an active filter and clears it", () => {
    expect(countActiveFilters({ salary_interval: "hour" })).toBe(1);
    expect(countActiveFilters({ salary_interval: null })).toBe(0);
    expect(matchesJob(hourly(4500, 5500), {})).toBe(true);
  });
});
