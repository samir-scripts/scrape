import { describe, it, expect } from "vitest";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import type { RoleType } from "../bindings/RoleType";
import type { Settings } from "../bindings/Settings";

describe("Domain models & types", () => {
  it("matches expected job status values", () => {
    const statuses: JobStatus[] = [
      "scraped",
      "viewed",
      "applied",
      "interviewing",
      "offer",
      "rejected",
      "ignored",
    ];
    expect(statuses).toHaveLength(7);
  });

  it("matches expected role types", () => {
    const roleTypes: RoleType[] = [
      "full_time",
      "contract",
      "internship",
      "part_time",
      "unknown",
    ];
    expect(roleTypes).toHaveLength(5);
  });

  it("validates a typed job object", () => {
    const job: Job = {
      id: "0192-test",
      dedupe_key: "greenhouse:stripe:101",
      source_id: null,
      external_id: "101",
      title: "Data Engineer",
      company: "Stripe",
      role_type: "full_time",
      location: "Remote",
      url: "https://stripe.com/jobs/101",
      date_posted: "2026-10-01T00:00:00Z",
      date_added: "2026-10-09T00:00:00Z",
      last_seen_at: "2026-10-09T00:00:00Z",
      status: "applied",
      status_changed_at: "2026-10-09T01:00:00Z",
      notes: "Referred by friend",
    };
    expect(job.company).toBe("Stripe");
    expect(job.status).toBe("applied");
  });

  it("validates typed settings", () => {
    const settings: Settings = {
      system_locale: "en-US",
      system_timezone: "America/New_York",
      system_week_start: "sunday",
      ui_theme: "paper",
      ui_reduce_motion: false,
      scraper_enabled: true,
      scraper_cron: "0 0 */6 * * *",
      scraper_run_on_startup: false,
      filters_include_keywords: ["engineer"],
      filters_exclude_keywords: [],
      filters_locations: ["remote"],
      filters_role_types: ["full_time"],
      notifications_enabled: true,
      app_start_hidden: true,
      app_launch_at_login: false,
    };
    expect(settings.ui_theme).toBe("paper");
    expect(settings.scraper_enabled).toBe(true);
  });
});
