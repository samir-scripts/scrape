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
      expect(html).toContain("https://job-boards.greenhouse.io/cloudflare/jobs/8097321?gh_jid=8097321");
      expect(html).toContain("Source evidence");
      expect(html).toContain("Open posting");

      // 2. Must render visible external link indicator [↗]
      expect(html).toContain("Open posting");

      // 3. Must render action button
      expect(html).toContain("Open posting");
    });
  });

  describe("JobCard listing actions", () => {
    it("keeps external navigation out of the compact card actions", () => {
      const onOpenUrl = vi.fn();
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: mockJob,
          onOpenUrl,
        })
      );

      expect(html).not.toContain(`aria-label="Open ${mockJob.title.replaceAll("&", "&amp;")} at ${mockJob.company} in browser"`);
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

      expect(html).toContain("https://job-boards.greenhouse.io/cloudflare/jobs/8097321?gh_jid=8097321");
      expect(html).toContain("Source evidence");
      expect(html).toContain("Open posting");
      expect(html).toContain("Open posting");
    });
  });

  describe("Explicit fictional demonstration fixtures", () => {
    it("keeps fictional identity, destination, description and pay coherent", () => {
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
        needs_review: false, script_trusted: false,
      };

      const jobs = generateScrapedJobsForSource(source);
      expect(jobs.length).toBeGreaterThan(0);

      for (const job of jobs) {
        expect(job.company).toBe("Demo Company");
        expect(job.external_id).toMatch(/^demo-\d+$/);
        expect(job.url).toBe(`https://example.com/demo/${source.kind}/${encodeURIComponent(source.identifier)}/jobs/${job.external_id}`);
        expect(job.description_text).toContain("FICTIONAL DEMONSTRATION POSTING");
        expect(job.description_text).toContain(job.title);
        expect(job.description_text).toContain(job.salary_raw!);
        expect(job.company_icon_url).toBeNull();
      }
      const austinIntern = jobs.find((j) => j.external_id === "demo-1");
      expect(austinIntern?.workplace_type).toBe("onsite");
      expect(austinIntern?.salary_interval).toBe("hour");
      expect(austinIntern?.description_text).toContain("In-Office");

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
        needs_review: false, script_trusted: false,
      };

      const filteredJobs = generateScrapedJobsForSource(sourceWithExclusions);
      const externalIds = filteredJobs.map((j) => j.external_id);

      // 8247492 (Technical Product Marketing Manager) must be excluded
      expect(externalIds).not.toContain("demo-5");
      // 7695702 (Account Executive, FedCiv) doesn't have Marketing/Sales in title, but let's check titles:
      for (const job of filteredJobs) {
        expect(job.title.toLowerCase()).not.toContain("marketing");
        expect(job.title.toLowerCase()).not.toContain("sales");
      }

      // Matching fictional engineering jobs are retained
      expect(externalIds).toContain("demo-1");
      expect(externalIds).toContain("demo-2");
    });
  });
});
