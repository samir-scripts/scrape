import { describe, it, expect } from "vitest";
import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";
import {
  matchesJob,
  formatSalary,
  formatExperience,
  formatWorkAuth,
  triStateToLabel,
  countActiveFilters,
} from "../lib/filter";
import { toggleRoleSelection } from "../components/filters/RoleTypeFilter";

const createMockJob = (overrides: Partial<Job> = {}): Job => ({
  id: "job-1",
  dedupe_key: "greenhouse:test:1",
  source_id: "src-1",
  external_id: "1",
  title: "Senior Software Engineer",
  company: "Acme Corp",
  role_type: "full_time",
  location: "San Francisco, CA",
  url: "https://example.com/jobs/1",
  date_posted: "2026-10-01T00:00:00Z",
  date_added: "2026-10-09T00:00:00Z",
  last_seen_at: "2026-10-09T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-09T00:00:00Z",
  notes: null,
  favorited: false,
  favorited_at: null,
  salary_min: 150000,
  salary_max: 200000,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$150,000 - $200,000",
  salary_source: "structured",
  workplace_type: "hybrid",
  sponsorship: "yes",
  sponsorship_evidence: "We sponsor H1B visas.",
  clearance: "no",
  clearance_level: null,
  clearance_evidence: "No clearance required.",
  work_auth: "open",
  work_auth_evidence: "We sponsor H1B visas.",
  experience_min: 5,
  experience_max: 8,
  experience_raw: "5-8 years",
  is_new_grad: "no",
  experience_evidence: "Minimum 5 years in Rust.",
  description_text: "Job description text...",
  company_icon_url: null,
  enrichment_version: 1,
  locations: [
    {
      raw: "San Francisco, CA, US",
      city: "San Francisco",
      region_code: "CA",
      country_code: "US",
      is_remote: false,
    },
  ],
  ...overrides,
});

describe("Job Filtering - matchesJob", () => {
  it("matches job within salary range overlap", () => {
    const job = createMockJob({ salary_min: 120000, salary_max: 180000 });

    // Filter [140k, 200k] overlaps [120k, 180k]
    expect(matchesJob(job, { salary_min: 140000, salary_max: 200000 })).toBe(true);

    // Filter [190k, 250k] does NOT overlap [120k, 180k]
    expect(matchesJob(job, { salary_min: 190000, salary_max: 250000 })).toBe(false);

    // Filter [50k, 100k] does NOT overlap [120k, 180k]
    expect(matchesJob(job, { salary_min: 50000, salary_max: 100000 })).toBe(false);
  });

  it("handles jobs with unknown salary based on salary_include_unknown toggle", () => {
    const job = createMockJob({ salary_min: null, salary_max: null });

    // When include_unknown is true (default), matches
    expect(
      matchesJob(job, { salary_min: 100000, salary_include_unknown: true })
    ).toBe(true);

    // When include_unknown is false, rejects
    expect(
      matchesJob(job, { salary_min: 100000, salary_include_unknown: false })
    ).toBe(false);
  });

  it("matches job within experience (YOE) bounds", () => {
    const job = createMockJob({ experience_min: 3, experience_max: 6 });

    // Filter max YOE 5 overlaps [3, 6]
    expect(matchesJob(job, { experience_min: 0, experience_max: 5 })).toBe(true);

    // Filter min YOE 7 does NOT overlap [3, 6]
    expect(matchesJob(job, { experience_min: 7, experience_max: 10 })).toBe(false);

    // Filter max YOE 2 does NOT overlap [3, 6]
    expect(matchesJob(job, { experience_min: 0, experience_max: 2 })).toBe(false);
  });

  it("handles New Grad tri-state filter", () => {
    const newGradJob = createMockJob({ is_new_grad: "yes" });
    const seniorJob = createMockJob({ is_new_grad: "no" });

    // "any" allows both
    expect(matchesJob(newGradJob, { new_grad: "any" })).toBe(true);
    expect(matchesJob(seniorJob, { new_grad: "any" })).toBe(true);

    // "only" permits only new grad jobs
    expect(matchesJob(newGradJob, { new_grad: "only" })).toBe(true);
    expect(matchesJob(seniorJob, { new_grad: "only" })).toBe(false);

    // "exclude" rejects new grad jobs
    expect(matchesJob(newGradJob, { new_grad: "exclude" })).toBe(false);
    expect(matchesJob(seniorJob, { new_grad: "exclude" })).toBe(true);
  });

  it("filters by visa sponsorship and clearance requirements", () => {
    const job = createMockJob({
      sponsorship: "yes",
      clearance: "no",
    });

    // Sponsorship filter includes "yes"
    expect(matchesJob(job, { sponsorship: ["yes"] })).toBe(true);
    // Sponsorship filter only wants "no"
    expect(matchesJob(job, { sponsorship: ["no"] })).toBe(false);

    // Clearance filter includes "no"
    expect(matchesJob(job, { clearance: ["no", "unknown"] })).toBe(true);
    // Clearance filter requires "yes"
    expect(matchesJob(job, { clearance: ["yes"] })).toBe(false);
  });

  it("filters by workplace type and geography", () => {
    const job = createMockJob({
      workplace_type: "hybrid",
      locations: [
        {
          raw: "Toronto, ON, CA",
          city: "Toronto",
          region_code: "ON",
          country_code: "CA",
          is_remote: false,
        },
      ],
    });

    // Workplace type
    expect(matchesJob(job, { remote: "hybrid" })).toBe(true);
    expect(matchesJob(job, { remote: "remote" })).toBe(false);

    // Country match
    expect(matchesJob(job, { countries: ["CA"] })).toBe(true);
    expect(matchesJob(job, { countries: ["US", "GB"] })).toBe(false);

    // Region match
    expect(matchesJob(job, { regions: ["ON"] })).toBe(true);
    expect(matchesJob(job, { regions: ["CA", "NY"] })).toBe(false);
  });
});

describe("Formatting Helpers", () => {
  it("formats salary values cleanly", () => {
    const job1 = createMockJob({
      salary_min: 120000,
      salary_max: 160000,
      salary_currency: "USD",
    });
    expect(formatSalary(job1)).toBe("$120K - $160K");

    const job2 = createMockJob({
      salary_min: 150000,
      salary_max: 150000,
      salary_currency: "USD",
    });
    expect(formatSalary(job2)).toBe("$150K");

    const job3 = createMockJob({
      salary_min: 80000,
      salary_max: null,
      salary_currency: "CAD",
    });
    expect(formatSalary(job3)).toBe("FROM CAD 80K");

    const job4 = createMockJob({
      salary_min: null,
      salary_max: null,
      salary_raw: "Competitive",
    });
    expect(formatSalary(job4)).toBe("Competitive");
  });

  it("formats experience values cleanly", () => {
    const job1 = createMockJob({ experience_min: 3, experience_max: 5 });
    expect(formatExperience(job1)).toBe("3-5 YOE");

    const job2 = createMockJob({ experience_min: 7, experience_max: null });
    expect(formatExperience(job2)).toBe("7+ YOE");

    const job3 = createMockJob({
      experience_min: null,
      experience_max: null,
      experience_raw: null,
    });
    expect(formatExperience(job3)).toBe("—");
  });

  it("formats TriState values to user-facing labels", () => {
    expect(triStateToLabel("yes")).toBe("Yes");
    expect(triStateToLabel("no")).toBe("No");
    expect(triStateToLabel("unknown")).toBe("N/A");
  });

  it("accurately counts active non-default filter dimensions", () => {
    const filter1: Partial<JobFilter> = {};
    expect(countActiveFilters(filter1)).toBe(0);

    const filter2: Partial<JobFilter> = {
      salary_min: 100000,
      new_grad: "only",
      sponsorship: ["yes"],
      role_types: ["full_time"],
    };
    expect(countActiveFilters(filter2)).toBe(4);
  });

  it("filters jobs by consolidated role_types filter spec", () => {
    const ftJob = createMockJob({ role_type: "full_time" });
    const contractJob = createMockJob({ role_type: "contract" });
    const internJob = createMockJob({ role_type: "internship" });
    const seasonalJob = createMockJob({ role_type: "seasonal" });

    const filter: Partial<JobFilter> = {
      role_types: ["full_time", "seasonal"],
    };

    expect(matchesJob(ftJob, filter)).toBe(true);
    expect(matchesJob(seasonalJob, filter)).toBe(true);
    expect(matchesJob(contractJob, filter)).toBe(false);
    expect(matchesJob(internJob, filter)).toBe(false);

    // Empty or null role_types matches all roles
    expect(matchesJob(internJob, { role_types: null })).toBe(true);
    expect(matchesJob(internJob, { role_types: [] })).toBe(true);
  });

  it("matches region codes flexibly across ISO and short abbreviations", () => {
    const usCaJob = createMockJob({
      locations: [
        {
          raw: "San Francisco, CA",
          city: "San Francisco",
          region_code: "US-CA",
          country_code: "US",
          is_remote: false,
        },
      ],
    });

    // Filtering by short code "CA" matches "US-CA"
    expect(matchesJob(usCaJob, { regions: ["CA"] })).toBe(true);
    // Filtering by full ISO code "US-CA" matches "US-CA"
    expect(matchesJob(usCaJob, { regions: ["US-CA"] })).toBe(true);
    // Filtering by "NY" does not match
    expect(matchesJob(usCaJob, { regions: ["NY"] })).toBe(false);
  });

  it("toggles role selections cleanly including seasonal", () => {
    // Starting with null, adding full_time
    const sel1 = toggleRoleSelection(null, "full_time");
    expect(sel1).toEqual(["full_time"]);

    // Adding seasonal
    const sel2 = toggleRoleSelection(sel1, "seasonal");
    expect(sel2).toEqual(["full_time", "seasonal"]);

    // Removing full_time leaves seasonal
    const sel3 = toggleRoleSelection(sel2, "full_time");
    expect(sel3).toEqual(["seasonal"]);

    // Removing seasonal resets to null (any)
    const sel4 = toggleRoleSelection(sel3, "seasonal");
    expect(sel4).toBeNull();
  });
});

describe("Anchored Filter Dropdown Coordinate & Viewport Geometry", () => {
  it("calculates anchored position directly below trigger button", async () => {
    const { calculateFilterDropdownCoordinates } = await import(
      "../components/filters/FilterPanel"
    );

    // Trigger button located at x: 140, y: 36, height: 28 (bottom: 64)
    const triggerRect = {
      top: 36,
      bottom: 64,
      left: 140,
      right: 230,
      width: 90,
    };

    const coords = calculateFilterDropdownCoordinates(
      triggerRect,
      1280,
      800,
      420,
      4
    );

    // top = bottom + gap = 64 + 4 = 68
    expect(coords.top).toBe(68);
    // left aligns with triggerRect.left
    expect(coords.left).toBe(140);
    expect(coords.width).toBe(420);
    // maxHeight = 800 - 68 - 16 = 716
    expect(coords.maxHeight).toBe(716);
  });

  it("clamps left edge when button is near right viewport boundary", async () => {
    const { calculateFilterDropdownCoordinates } = await import(
      "../components/filters/FilterPanel"
    );

    // Button near right edge: left 950 on 1024px screen
    // Panel width 420px. maxLeft = 1024 - 420 - 12 = 592
    const triggerRect = {
      top: 36,
      bottom: 64,
      left: 950,
      right: 1010,
      width: 60,
    };

    const coords = calculateFilterDropdownCoordinates(
      triggerRect,
      1024,
      768,
      420,
      4
    );

    expect(coords.left).toBe(592);
    // Dropdown right boundary: 592 + 420 = 1012 (12px padding from viewport edge)
    expect(coords.left + coords.width).toBe(1012);
  });

  it("provides safe fallback geometry when trigger element is unmeasured", async () => {
    const { calculateFilterDropdownCoordinates } = await import(
      "../components/filters/FilterPanel"
    );

    const coords = calculateFilterDropdownCoordinates(null, 1024, 768);

    expect(coords.top).toBe(48);
    expect(coords.left).toBe(12);
    expect(coords.width).toBe(420);
    expect(coords.maxHeight).toBe(704);
  });
});

describe("Work Authorization & Green Card Filtering", () => {
  it("formats work authorization requirements accurately", () => {
    expect(formatWorkAuth(createMockJob({ work_auth: "us_person" }))).toBe("GC / US PERSON");
    expect(formatWorkAuth(createMockJob({ work_auth: "citizen_only" }))).toBe("CITIZEN ONLY");
    expect(formatWorkAuth(createMockJob({ work_auth: "open" }))).toBe("OPEN");
    expect(formatWorkAuth(createMockJob({ work_auth: "unknown" }))).toBe("—");
  });

  it("filters jobs based on work_auth requirements array", () => {
    const usPersonJob = createMockJob({ work_auth: "us_person" });
    const citizenJob = createMockJob({ work_auth: "citizen_only" });
    const openJob = createMockJob({ work_auth: "open" });

    // Filter for US Person (Green card eligible)
    const usPersonFilter: JobFilter = {
      statuses: null,
      role_types: null,
      search: null,
      source_id: null,
      sort: null,
      limit: null,
      offset: null,
      favorited_only: null,
      salary_min: null,
      salary_max: null,
      salary_currency: null,
      salary_include_unknown: true,
      salary_interval: null,
      experience_min: null,
      experience_max: null,
      experience_include_unknown: true,
      experience_match_mode: "advertised_range_overlap",
      new_grad: null,
      sponsorship: null,
      clearance: null,
      countries: null,
      regions: null,
      cities: null,
      remote: null,
      location_include_unknown: true,
      work_auth: ["us_person", "open"],
    };

    expect(matchesJob(usPersonJob, usPersonFilter)).toBe(true);
    expect(matchesJob(openJob, usPersonFilter)).toBe(true);
    expect(matchesJob(citizenJob, usPersonFilter)).toBe(false);
  });

  it("filters jobs based on cities", () => {
    const lisbonJob = createMockJob({
      location: "Lisbon, Portugal",
      locations: [{ raw: "Lisbon, Portugal", city: "Lisbon", region_code: null, country_code: "PT", is_remote: false }],
    });
    const sfJob = createMockJob({
      location: "San Francisco, CA",
      locations: [{ raw: "San Francisco, CA", city: "San Francisco", region_code: "CA", country_code: "US", is_remote: false }],
    });

    expect(matchesJob(lisbonJob, { cities: ["Lisbon"] })).toBe(true);
    expect(matchesJob(sfJob, { cities: ["Lisbon"] })).toBe(false);
    expect(matchesJob(sfJob, { cities: ["San Francisco"] })).toBe(true);
  });
});

