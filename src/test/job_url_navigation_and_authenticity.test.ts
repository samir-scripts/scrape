import { describe, it, expect, vi } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { Job } from "../bindings/Job";
import type { Source } from "../bindings/Source";
import { JobDetail } from "../components/views/discovered/JobDetail";
import { JobCard } from "../components/views/discovered/JobCard";
import { JobDetailDrawer } from "../components/views/tracked/JobDetailDrawer";
import { generateScrapedJobsForSource } from "../lib/api";

const mockJob: Job = {
  id: "job-cf-8097321",
  dedupe_key: "gh:cloudflare:ai-sec-8097321",
  source_id: "src-cf",
  external_id: "8097321",
  title: "AI Security Research & Red Team Engineer",
  company: "Cloudflare",
  role_type: "full_time",
  location: "San Francisco, CA",
  url: "https://job-boards.greenhouse.io/cloudflare/jobs/8097321?gh_jid=8097321",
  date_posted: "2026-10-10T00:00:00Z",
  date_added: "2026-10-10T12:00:00Z",
  last_seen_at: "2026-10-11T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-10T12:00:00Z",
  notes: null,
  favorited: false,
  favorited_at: null,
  salary_min: 160000,
  salary_max: 210000,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$160k - $210k",
  salary_source: "structured",
  workplace_type: "hybrid",
  sponsorship: "yes",
  sponsorship_evidence: "Sponsors visas",
  clearance: "no",
  clearance_level: null,
  clearance_evidence: null,
  work_auth: "open",
  work_auth_evidence: null,
  experience_min: 5,
  experience_max: 8,
  experience_raw: "5-8 years",
  is_new_grad: "no",
  experience_evidence: "5+ years security research",
  description_text: "Join Cloudflare AI security research team.",
  company_icon_url: "https://cloudflare.com/favicon.ico",
  enrichment_version: 1,
  locations: [],
};

describe("Job URL Navigation & Authenticity Engine", () => {
  describe("Consolidated JobDetail Window Link Rendering", () => {
    it("renders job.url as an active HTML anchor link with target=_blank and rel attributes", () => {
      const onOpenUrl = vi.fn();
      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: mockJob,
          onOpenUrl,
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
        })
      );

      // 1. Must render as an <a> tag pointing to the genuine job URL
      expect(html).toContain('href="https://job-boards.greenhouse.io/cloudflare/jobs/8097321?gh_jid=8097321"');
      expect(html).toContain('target="_blank"');
      expect(html).toContain('rel="noopener noreferrer"');

      // 2. Must render visible external link indicator [↗]
      expect(html).toContain("[↗]");

      // 3. Must render action button
      expect(html).toContain("OPEN POSTING");
    });
  });

  describe("JobCard External Navigation Action", () => {
    it("renders an external link button on the card with onOpenUrl handler", () => {
      const onOpenUrl = vi.fn();
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: mockJob,
          onOpenUrl,
        })
      );

      // Card must render external open button [↗]
      expect(html).toContain("[↗]");
      expect(html).toContain("Open job posting in browser");
    });
  });

  describe("Tracked JobDetailDrawer URL Bar", () => {
    it("renders interactive anchor link bar in the drawer", () => {
      const onOpenUrl = vi.fn();
      const html = renderToStaticMarkup(
        React.createElement(JobDetailDrawer, {
          job: mockJob,
          isOpen: true,
          onClose: () => {},
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl,
        })
      );

      expect(html).toContain('href="https://job-boards.greenhouse.io/cloudflare/jobs/8097321?gh_jid=8097321"');
      expect(html).toContain('target="_blank"');
      expect(html).toContain("[↗]");
      expect(html).toContain("OPEN [↵]");
    });
  });

  describe("Authentic Job IDs in Dev/Seed Workflow Generator", () => {
    it("generates authentic job-boards.greenhouse.io URLs with verified IDs instead of dummy 1000 indices", () => {
      const source: Source = {
        id: "src-cf",
        kind: "greenhouse",
        name: "Cloudflare — Software Engineering & Internships",
        identifier: "cloudflare",
        config: { url: "https://boards.greenhouse.io/cloudflare" },
        enabled: true,
        last_run_at: null,
        last_status: null,
        last_error: null,
        created_at: "2026-10-10T00:00:00Z",
        cadence: "daily",
        cadence_cron: null,
        next_run_at: null,
        last_run_duration_ms: null,
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
        filters: null,
      };

      const jobs = generateScrapedJobsForSource(source);
      expect(jobs.length).toBeGreaterThan(0);

      for (const job of jobs) {
        // Must never generate dummy /jobs/1000 that redirects to cloudflare.com/careers
        expect(job.url).not.toContain("/jobs/1000");
        expect(job.url).not.toContain("/jobs/1001");
        expect(job.url).not.toContain("/jobs/1002");
        expect(job.url).not.toContain("/jobs/1003");

        // Must point to job-boards.greenhouse.io or config base with real external id
        expect(job.url).toContain("cloudflare");
        expect(job.url).toContain(job.external_id!);
      }

      // Verify specific active job IDs are present
      const externalIds = jobs.map((j) => j.external_id);
      expect(externalIds).toContain("8199958");
      expect(externalIds).toContain("8245197");
      expect(externalIds).toContain("8097321");
      expect(externalIds).toContain("7742773");
      expect(externalIds).toContain("8247492");

      // Verify authentic title matches actual Greenhouse board for ID 8247492
      const marketingJob = jobs.find((j) => j.external_id === "8247492");
      expect(marketingJob).toBeDefined();
      expect(marketingJob?.title).toBe("Technical Product Marketing Manager");
      expect(marketingJob?.url).toBe(
        "https://boards.greenhouse.io/cloudflare/jobs/8247492?gh_jid=8247492"
      );
      expect(marketingJob?.workplace_type).toBe("remote");

      // Verify In-Office internship roles are classified as onsite (not hybrid or remote)
      const austinIntern = jobs.find((j) => j.external_id === "8199958");
      expect(austinIntern?.workplace_type).toBe("onsite");
      expect(austinIntern?.description_text).toContain("In-Office");

      const lisbonIntern = jobs.find((j) => j.external_id === "8245197");
      expect(lisbonIntern?.workplace_type).toBe("onsite");

      const londonIntern = jobs.find((j) => j.external_id === "8245211");
      expect(londonIntern?.workplace_type).toBe("onsite");

      // Verify hybrid engineering role
      const hybridEng = jobs.find((j) => j.external_id === "8187404");
      expect(hybridEng?.workplace_type).toBe("hybrid");
    });

    it("renders IN-OFFICE tag on JobCard when workplace_type is onsite, never Remote", () => {
      const onsiteJob: Job = {
        ...mockJob,
        workplace_type: "onsite",
        location: "Austin, TX",
      };
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: onsiteJob,
          onOpenUrl: () => {},
        })
      );

      expect(html).toContain("IN-OFFICE");
      expect(html).not.toContain("Remote");
    });

    it("filters out excluded jobs when exclude_keywords is configured in the source", () => {
      const sourceWithExclusions: Source = {
        id: "src-cf-filtered",
        kind: "greenhouse",
        name: "Cloudflare — Software Engineering",
        identifier: "cloudflare",
        config: { url: "https://boards.greenhouse.io/cloudflare" },
        enabled: true,
        last_run_at: null,
        last_status: null,
        last_error: null,
        created_at: "2026-10-10T00:00:00Z",
        cadence: "daily",
        cadence_cron: null,
        next_run_at: null,
        last_run_duration_ms: null,
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
        filters: {
          exclude_keywords: ["Marketing", "Sales"],
        },
      };

      const filteredJobs = generateScrapedJobsForSource(sourceWithExclusions);
      const externalIds = filteredJobs.map((j) => j.external_id);

      // 8247492 (Technical Product Marketing Manager) must be excluded
      expect(externalIds).not.toContain("8247492");
      // 7695702 (Account Executive, FedCiv) doesn't have Marketing/Sales in title, but let's check titles:
      for (const job of filteredJobs) {
        expect(job.title.toLowerCase()).not.toContain("marketing");
        expect(job.title.toLowerCase()).not.toContain("sales");
      }

      // Authentic engineering jobs are retained
      expect(externalIds).toContain("8199958");
      expect(externalIds).toContain("8245197");
    });
  });
});
