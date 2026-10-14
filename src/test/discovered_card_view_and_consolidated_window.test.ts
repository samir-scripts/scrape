import { describe, it, expect, vi } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { Job } from "../bindings/Job";
import { JobCard } from "../components/views/discovered/JobCard";
import { DiscoveredTwoColumn } from "../components/layout/DiscoveredTwoColumn";
import { JobDetail } from "../components/views/discovered/JobDetail";

const mockJob: Job = {
  id: "job-stripe-101",
  dedupe_key: "gh:stripe:eng-101",
  source_id: "src-1",
  external_id: "101",
  title: "Senior Staff Infrastructure Engineer",
  company: "Stripe",
  role_type: "full_time",
  location: "Remote, US",
  url: "https://stripe.com/jobs/101",
  date_posted: "2026-08-05T00:00:00Z",
  date_added: "2026-08-05T12:00:00Z",
  last_seen_at: "2026-08-06T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-08-05T12:00:00Z",
  notes: "Referral from teammate",
  favorited: true,
  favorited_at: "2026-08-05T12:00:00Z",
  salary_min: 190000,
  salary_max: 260000,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$190k - $260k",
  salary_source: "structured",
  workplace_type: "remote",
  sponsorship: "yes",
  sponsorship_evidence: "Sponsors H-1B visas and green cards.",
  clearance: "no",
  clearance_level: null,
  clearance_evidence: null,
  work_auth: "us_person",
  work_auth_evidence: "Must be a US Citizen or Permanent Resident (Green Card holder).",
  experience_min: 6,
  experience_max: null,
  experience_raw: "6+ years",
  is_new_grad: "no",
  experience_evidence: "6+ years of distributed systems engineering.",
  description_text: "Lead critical infrastructure systems scaling globally across multi-region cloud clusters.",
  company_icon_url: "https://stripe.com/favicon.ico",
  enrichment_version: 1,
  locations: [],
};

describe("Discovered Tab Card View and Consolidated Job Window", () => {
  describe("JobCard Component", () => {
    it("renders job title, company name, formatted date, and semantic remote badge", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: mockJob,
          selected: false,
          onToggleFavorite: () => {},
        })
      );

      // 1. Job Title & Company
      expect(html).toContain("Senior Staff Infrastructure Engineer");
      expect(html).toContain("Stripe");

      // 2. Formatted date on card: Aug 5, 2026
      expect(html).toContain("Aug 5, 2026");

      // 3. Remote badge uses semantic theme colors
      expect(html).toContain("Remote");
      expect(html).toContain("text-[var(--color-success)]");
      expect(html).toContain("border-current");

      // 4. Badges row
      expect(html).toContain("$190K - $260K");
      expect(html).toContain("6+ YOE");
      expect(html).toContain("full time");
      expect(html).toContain("SPONSORS");
      expect(html).toContain("GC / US PERSON");

      // 5. Description snippet preview
      expect(html).toContain("Lead critical infrastructure systems scaling");

      // 6. 1px card border
      expect(html).toContain("border-[var(--color-ink)]/20");
    });

    it("renders selected card accent border when selected is true", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: mockJob,
          selected: true,
        })
      );

      expect(html).toContain("border-l-4");
      expect(html).toContain("border-l-[var(--color-ink)]");
    });
  });

  describe("DiscoveredTwoColumn Layout", () => {
    it("renders 1-pane card stream when right (job window) is null", () => {
      const html = renderToStaticMarkup(
        React.createElement(DiscoveredTwoColumn, {
          left: React.createElement("div", { id: "card-stream" }, "CARD STREAM ITEMS"),
          right: null,
        })
      );

      // In 1-pane mode, left is centered in max-w-4xl container
      expect(html).toContain("CARD STREAM ITEMS");
      expect(html).toContain("max-w-4xl");
      // Separator and right pane must NOT be rendered
      expect(html).not.toContain("role=\"separator\"");
      expect(html).not.toContain("JOB DETAILS");
    });

    it("renders 2-pane view with resizable separator when right is provided", () => {
      const html = renderToStaticMarkup(
        React.createElement(DiscoveredTwoColumn, {
          left: React.createElement("div", { id: "card-stream" }, "CARD STREAM ITEMS"),
          right: React.createElement("div", { id: "job-detail" }, "CONSOLIDATED JOB WINDOW"),
        })
      );

      // Both panes rendered
      expect(html).toContain("CARD STREAM ITEMS");
      expect(html).toContain("CONSOLIDATED JOB WINDOW");

      // Resizable separator rendered with accessibility attributes
      expect(html).toContain("role=\"separator\"");
      expect(html).toContain("aria-orientation=\"vertical\"");
    });
  });

  describe("Consolidated JobDetail Window", () => {
    it("consolidates status, metadata, triage actions, and description into single window", () => {
      const onClose = vi.fn();
      const onUpdateStatus = vi.fn();
      const onUpdateNotes = vi.fn();
      const onOpenUrl = vi.fn();
      const onToggleFavorite = vi.fn();

      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: mockJob,
          onClose,
          onUpdateStatus,
          onUpdateNotes,
          onOpenUrl,
          onToggleFavorite,
        })
      );

      expect(html).toContain('aria-label="Close job details"');
      expect(html).toContain('Open posting');
      expect(html).toContain('Mark as applied');
      expect(html).toContain('More details');
      expect(html).toContain('Source evidence');
      expect(html).toContain('Sponsors H-1B visas and green cards.');
      expect(html).toContain('Lead critical infrastructure systems');
      expect(html).toContain('Notes');
    });
  });
});
