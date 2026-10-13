import { describe, it, expect } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { api } from "../lib/api";
import type { Job } from "../bindings/Job";
import { FavoriteButton } from "../components/ui/FavoriteButton";
import { CompanyLogo } from "../components/ui/CompanyLogo";
import { ThreeColumn } from "../components/layout/ThreeColumn";
import {
  DiscoveredTable,
  compareDiscoveredJobs,
} from "../components/views/discovered/DiscoveredTable";

const mockJob: Job = {
  id: "job-stripe-101",
  dedupe_key: "gh:stripe:eng-101",
  source_id: "src-1",
  external_id: "101",
  title: "Senior Staff Infrastructure Engineer",
  company: "Stripe",
  role_type: "full_time",
  location: "San Francisco, CA",
  url: "https://stripe.com/jobs/101",
  date_posted: "2026-10-10T00:00:00Z",
  date_added: "2026-10-11T00:00:00Z",
  last_seen_at: "2026-10-11T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-11T00:00:00Z",
  notes: null,
  favorited: false,
  favorited_at: null,
  salary_min: 190000,
  salary_max: 260000,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$190k - $260k",
  salary_source: "structured",
  workplace_type: "hybrid",
  sponsorship: "yes",
  sponsorship_evidence: "Sponsors visas",
  clearance: "no",
  clearance_level: null,
  clearance_evidence: null,
  work_auth: "us_person",
  work_auth_evidence: "Must be a US Person (Citizen or Permanent Resident)",
  experience_min: 6,
  experience_max: null,
  experience_raw: "6+ years",
  is_new_grad: "no",
  experience_evidence: "6+ years in distributed systems",
  description_text: "Lead critical infrastructure systems scaling globally across multi-region clusters.",
  company_icon_url: "https://stripe.com/favicon.ico",
  enrichment_version: 1,
  locations: [],
};

describe("Deduplication Idempotency (Workflow Re-Runs)", () => {
  it("does not duplicate jobs when the same workflow/source is run multiple times", async () => {
    // 1. Get initial sources
    const sources = await api.listSources();
    expect(sources.length).toBeGreaterThan(0);
    const targetSource = sources[0];

    // 2. First execution
    const run1 = await api.runSourceNow(targetSource.id);
    expect(run1).toBeDefined();

    const jobsAfterFirstRun = await api.listJobs({ limit: 100 });
    const countAfterFirstRun = jobsAfterFirstRun.total;

    // Dedupe keys should be deterministic
    const sourceJobs1 = jobsAfterFirstRun.items.filter((j) => j.source_id === targetSource.id);
    expect(sourceJobs1.length).toBeGreaterThan(0);
    const dedupeKeys1 = sourceJobs1.map((j) => j.dedupe_key);
    const uniqueKeys1 = new Set(dedupeKeys1);
    expect(uniqueKeys1.size).toBe(dedupeKeys1.length); // All keys are unique

    // 3. Mark one job as applied and favorited to verify user triage preservation
    const sampleJob = sourceJobs1[0];
    await api.updateJobStatus(sampleJob.id, "applied");
    await api.setJobFavorite(sampleJob.id, true);

    const verifiedTriaged = await api.getJob(sampleJob.id);
    expect(verifiedTriaged.status).toBe("applied");
    expect(verifiedTriaged.favorited).toBe(true);

    // 4. Second execution of the exact same workflow/source
    const run2 = await api.runSourceNow(targetSource.id);
    expect(run2).toBeDefined();
    // In an idempotent second run with unchanged sources, new jobs discovered should be 0
    expect(run2.jobs_new).toBe(BigInt(0));

    // 5. Verify total jobs count has NOT inflated / duplicated
    const jobsAfterSecondRun = await api.listJobs({ limit: 100 });
    expect(jobsAfterSecondRun.total).toBe(countAfterFirstRun);

    const sourceJobs2 = jobsAfterSecondRun.items.filter((j) => j.source_id === targetSource.id);
    expect(sourceJobs2.length).toBe(sourceJobs1.length);

    // 6. Verify user triage (applied status, favorited) is strictly preserved!
    const jobAfterSecondRun = await api.getJob(sampleJob.id);
    expect(jobAfterSecondRun.status).toBe("applied");
    expect(jobAfterSecondRun.favorited).toBe(true);
  });
});

describe("Prominent Favorite Button Component", () => {
  it("renders a brutalist 5-point SVG star with proper accessible attributes", () => {
    const htmlUnfav = renderToStaticMarkup(
      React.createElement(FavoriteButton, {
        favorited: false,
        size: "md",
      })
    );

    expect(htmlUnfav).toContain('aria-pressed="false"');
    expect(htmlUnfav).toContain('aria-label="Favorite posting"');
    expect(htmlUnfav).toContain("<svg");
    expect(htmlUnfav).toContain("<polygon");
    expect(htmlUnfav).toContain('fill="none"');

    const htmlFav = renderToStaticMarkup(
      React.createElement(FavoriteButton, {
        favorited: true,
        size: "lg",
      })
    );

    expect(htmlFav).toContain('aria-pressed="true"');
    expect(htmlFav).toContain('aria-label="Unfavorite posting"');
    expect(htmlFav).toContain('fill="var(--color-lime)"');
    expect(htmlFav).toContain("w-7 h-7");
  });
});

describe("Company Logo & Monogram Fallback Component", () => {
  it("renders an img tag when an iconUrl is provided", () => {
    const html = renderToStaticMarkup(
      React.createElement(CompanyLogo, {
        company: "Stripe",
        iconUrl: "https://stripe.com/favicon.ico",
        size: "md",
      })
    );

    expect(html).toContain("<img");
    expect(html).toContain('src="https://stripe.com/favicon.ico"');
    expect(html).toContain('alt="Stripe"');
  });

  it("uses a neutral monogram when a known company has no provider logo", () => {
    const html = renderToStaticMarkup(
      React.createElement(CompanyLogo, {
        company: "Cloudflare",
        size: "md",
      })
    );

    expect(html).not.toContain("<img");
    expect(html).toContain("CL");
  });

  it("renders a brutalist monogram when no URL can be derived", () => {
    const html = renderToStaticMarkup(
      React.createElement(CompanyLogo, {
        company: "UnknownCorp",
        size: "sm",
      })
    );

    // Monogram fallback container
    expect(html).toContain("UN");
    expect(html).toContain("font-mono font-bold");
  });
});

describe("Resizable ThreeColumn Window Panes", () => {
  it("renders interactive window border dividers with ARIA separator attributes", () => {
    const html = renderToStaticMarkup(
      React.createElement(ThreeColumn, {
        left: React.createElement("div", null, "Left Pane Content"),
        center: React.createElement("div", null, "Center Pane Content"),
        right: React.createElement("div", null, "Right Pane Content"),
      })
    );

    expect(html).toContain("Left Pane Content");
    expect(html).toContain("Center Pane Content");
    expect(html).toContain("Right Pane Content");

    // Both interactive dividers should be rendered
    expect(html).toContain('role="separator"');
    expect(html).toContain('aria-label="Resize job list pane"');
    expect(html).toContain('aria-label="Resize metadata panel"');
    expect(html).toContain('aria-orientation="vertical"');
    expect(html).toContain("cursor-col-resize");
  });
});

describe("Discovered Table View & Description Column", () => {
  const jobs: Job[] = [
    mockJob,
    {
      ...mockJob,
      id: "job-cloudflare-202",
      dedupe_key: "gh:cloudflare:sys-202",
      title: "Systems Software Engineer",
      company: "Cloudflare",
      salary_min: 150000,
      salary_max: 190000,
      description_text: "Scale world-class edge proxy and edge DNS systems across hundreds of data centers.",
      favorited: true,
      status: "viewed",
    },
  ];

  it("renders DiscoveredTable with DESCRIPTION & RELEVANT INFO column", () => {
    const html = renderToStaticMarkup(
      React.createElement(DiscoveredTable, {
        jobs,
        selectedJobId: "job-stripe-101",
        onSelectJob: () => {},
        onUpdateStatus: () => {},
        onOpenUrl: () => {},
        onToggleFavorite: () => {},
      })
    );

    // Headers
    expect(html).toContain("COMPANY");
    expect(html).toContain("JOB TITLE");
    expect(html).toContain("STATUS");
    expect(html).toContain("GREEN CARD");
    expect(html).toContain("DESCRIPTION &amp; RELEVANT INFO");
    expect(html).toContain("SALARY");
    expect(html).toContain("TRIAGE");

    // Content snippets
    expect(html).toContain("Stripe");
    expect(html).toContain("Cloudflare");
    expect(html).toContain("Senior Staff Infrastructure Engineer");
    expect(html).toContain("Lead critical infrastructure systems scaling globally");
    expect(html).toContain("Scale world-class edge proxy and edge DNS systems");
  });

  it("compares and sorts discovered jobs correctly", () => {
    // Sort by company ascending
    const sortedCompanyAsc = [...jobs].sort((a, b) => compareDiscoveredJobs(a, b, "company", true));
    expect(sortedCompanyAsc[0].company).toBe("Cloudflare");
    expect(sortedCompanyAsc[1].company).toBe("Stripe");

    // Sort by company descending
    const sortedCompanyDesc = [...jobs].sort((a, b) => compareDiscoveredJobs(a, b, "company", false));
    expect(sortedCompanyDesc[0].company).toBe("Stripe");

    // Since compareDiscoveredJobs returns cmp:
    const cmpAsc = compareDiscoveredJobs(jobs[0], jobs[1], "company", true);
    expect(cmpAsc).toBeGreaterThan(0); // "Stripe" > "Cloudflare"

    // Sort by salary
    const cmpSalary = compareDiscoveredJobs(jobs[0], jobs[1], "salary", true);
    expect(cmpSalary).toBeGreaterThan(0); // Stripe salary (190k) > Cloudflare salary (150k)

    // Sort by favorited
    const cmpFav = compareDiscoveredJobs(jobs[0], jobs[1], "favorited", true);
    expect(cmpFav).toBeLessThan(0); // false (0) < true (1)
  });
});
