import { describe, it, expect } from "vitest";
import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";
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

describe("Hourly vs Annual Salary Parsing and Formatting", () => {
  it("parses hourly inputs correctly without multiplying by 1000", () => {
    expect(parseHourlyInput("45")).toBe(45);
    expect(parseHourlyInput("$60/hr")).toBe(60);
    expect(parseHourlyInput("125")).toBe(125);
    expect(parseHourlyInput("")).toBeNull();
    expect(parseHourlyInput("invalid")).toBeNull();
  });

  it("parses annual inputs correctly with k suffix or <= 500 heuristic", () => {
    expect(parseSalaryInput("150k")).toBe(150000);
    expect(parseSalaryInput("150")).toBe(150000);
    expect(parseSalaryInput("150000")).toBe(150000);
  });

  it("formats hourly interval jobs as $/HR", () => {
    const hourlyJob = createMockJob({
      salary_min: 93600, // 45 * 2080
      salary_max: 124800, // 60 * 2080
      salary_interval: "hour",
      salary_raw: "$45 - $60/hr",
    });
    expect(formatSalary(hourlyJob)).toBe("$45 - $60/HR");

    const singleHourly = createMockJob({
      salary_min: 104000,
      salary_max: 104000,
      salary_interval: "hour",
    });
    expect(formatSalary(singleHourly)).toBe("$50/HR");

    const minOnlyHourly = createMockJob({
      salary_min: 83200, // 40 * 2080
      salary_max: null,
      salary_interval: "hour",
    });
    expect(formatSalary(minOnlyHourly)).toBe("FROM $40/HR");

    const maxOnlyHourly = createMockJob({
      salary_min: null,
      salary_max: 145600, // 70 * 2080
      salary_interval: "hour",
    });
    expect(formatSalary(maxOnlyHourly)).toBe("UP TO $70/HR");
  });

  it("filters jobs by salary_interval", () => {
    const hourlyJob = createMockJob({ salary_interval: "hour" });
    const annualJob = createMockJob({ salary_interval: "year" });

    expect(matchesJob(hourlyJob, { salary_interval: "hour" })).toBe(true);
    expect(matchesJob(annualJob, { salary_interval: "hour" })).toBe(false);
  });

  it("increments countActiveFilters when salary_interval is set", () => {
    expect(countActiveFilters({ salary_interval: "hour" })).toBe(1);
    expect(countActiveFilters({ salary_interval: null })).toBe(0);
  });

  it("matches hourly job when filter bounds match equivalent annualized rate", () => {
    // $50/hr = $104,000/yr
    const hourlyJob = createMockJob({
      salary_min: 104000,
      salary_max: 104000,
      salary_interval: "hour",
    });

    // Filtering $40/hr ($83,200) to $60/hr ($124,800)
    const filter: Partial<JobFilter> = {
      salary_min: 83200,
      salary_max: 124800,
      salary_interval: "hour",
    };
    expect(matchesJob(hourlyJob, filter)).toBe(true);

    // Filtering $60/hr ($124,800) to $80/hr ($166,400)
    const outOfBoundsFilter: Partial<JobFilter> = {
      salary_min: 124800,
      salary_max: 166400,
      salary_interval: "hour",
    };
    expect(matchesJob(hourlyJob, outOfBoundsFilter)).toBe(false);
  });
});
