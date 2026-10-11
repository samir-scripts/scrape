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
      favorited: true,
      favorited_at: "2026-10-09T01:00:00Z",
      salary_min: 140000,
      salary_max: 180000,
      salary_currency: "USD",
      salary_interval: "year",
      salary_raw: "$140,000 - $180,000",
      salary_source: "structured",
      workplace_type: "remote",
      sponsorship: "yes",
      sponsorship_evidence: "Visa sponsorship available",
      clearance: "no",
      clearance_level: null,
      clearance_evidence: null,
      experience_min: 3,
      experience_max: 5,
      experience_raw: "3-5 years",
      is_new_grad: "no",
      experience_evidence: "Requires 3+ years experience",
      description_text: "Join Stripe as a Data Engineer...",
      enrichment_version: 1,
      locations: [
        {
          raw: "Remote, US",
          city: null,
          region_code: null,
          country_code: "US",
          is_remote: true,
        },
      ],
    };
    expect(job.company).toBe("Stripe");
    expect(job.status).toBe("applied");
  });

  it("validates typed settings", () => {
    const settings: Settings = {
      system_locale: "en-US",
      system_timezone: "America/New_York",
      system_week_start: "sunday",
      system_time_format: "12h",
      system_date_format: "MM/DD/YYYY",
      ui_theme: "paper",
      ui_reduce_motion: false,
      ui_tooltips_enabled: true,
      ui_font_family: "default",
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
      app_auto_update: true,
      app_update_channel: "stable",
      app_last_update_check: null,
    };
    expect(settings.ui_theme).toBe("paper");
    expect(settings.ui_tooltips_enabled).toBe(true);
    expect(settings.ui_font_family).toBe("default");
    expect(settings.scraper_enabled).toBe(true);
    expect(settings.app_auto_update).toBe(true);
    expect(settings.app_update_channel).toBe("stable");
  });

  it("updates dev settings correctly with dot-notation keys", async () => {
    const { api } = await import("../lib/api");

    const updatedTheme = await api.updateSetting("ui.theme", "obsidian");
    expect(updatedTheme.ui_theme).toBe("obsidian");

    const updatedTooltips = await api.updateSetting("ui.tooltips_enabled", false);
    expect(updatedTooltips.ui_tooltips_enabled).toBe(false);

    const updatedFont = await api.updateSetting("ui.font_family", "jetbrains");
    expect(updatedFont.ui_font_family).toBe("jetbrains");

    const updatedTime = await api.updateSetting("system.time_format", "24h");
    expect(updatedTime.system_time_format).toBe("24h");

    const updatedDate = await api.updateSetting("system.date_format", "DD/MM/YYYY");
    expect(updatedDate.system_date_format).toBe("DD/MM/YYYY");

    const updatedApp = await api.updateSetting("app.launch_at_login", true);
    expect(updatedApp.app_launch_at_login).toBe(true);

    const updatedNotif = await api.updateSetting("notifications.enabled", false);
    expect(updatedNotif.notifications_enabled).toBe(false);

    // Verify getSettings reflects the changes
    const current = await api.getSettings();
    expect(current.ui_theme).toBe("obsidian");
    expect(current.system_time_format).toBe("24h");
    expect(current.system_date_format).toBe("DD/MM/YYYY");
    expect(current.app_launch_at_login).toBe(true);
    expect(current.notifications_enabled).toBe(false);

    // Revert back for test cleanliness
    await api.updateSetting("ui.theme", "paper");
    await api.updateSetting("system.time_format", "12h");
    await api.updateSetting("system.date_format", "MM/DD/YYYY");
  });
});
