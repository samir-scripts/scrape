import { describe, it, expect } from "vitest";
import {
  STATUS_CONFIG,
  TRACKED_STAGES,
  NAV_TABS,
  ROLE_TYPES,
  ROLE_TYPE_LABELS,
  CRON_PRESETS,
} from "../lib/constants";
import type { JobStatus } from "../bindings/JobStatus";

describe("Frontend Constants & Themes", () => {
  const allStatuses: JobStatus[] = [
    "scraped",
    "viewed",
    "applied",
    "interviewing",
    "offer",
    "rejected",
    "ignored",
  ];

  it("STATUS_CONFIG maps all job statuses", () => {
    for (const status of allStatuses) {
      const config = STATUS_CONFIG[status];
      expect(config).toBeDefined();
      expect(config.label).toBeTruthy();
      expect(config.label).toBe(config.label.toUpperCase());
      expect(config.bg).toBeTruthy();
      expect(config.text).toBeTruthy();
      expect(config.border).toBeTruthy();
    }
  });

  it("STATUS_CONFIG adheres to styling specifications", () => {
    expect(STATUS_CONFIG.scraped.bg).toBe("var(--color-lavender)");
    expect(STATUS_CONFIG.viewed.bg).toBe("var(--color-cream)");
    expect(STATUS_CONFIG.applied.bg).toBe("var(--color-stage-applied)");
    expect(STATUS_CONFIG.interviewing.bg).toBe("var(--color-stage-interviewing)");
    expect(STATUS_CONFIG.offer.bg).toBe("var(--color-stage-offer)");
    expect(STATUS_CONFIG.rejected.bg).toBe("var(--color-stage-rejected)");
    expect(TRACKED_STAGES.map(stage => STATUS_CONFIG[stage].stageAccent)).toEqual([
      "var(--color-stage-applied)", "var(--color-stage-interviewing)",
      "var(--color-stage-offer)", "var(--color-stage-rejected)",
    ]);
    expect(STATUS_CONFIG.ignored.bg).toBe("transparent");
    expect(STATUS_CONFIG.ignored.dither).toBe("checker");
  });

  it("TRACKED_STAGES contains exactly 4 pipeline stages in order", () => {
    expect(TRACKED_STAGES).toEqual([
      "applied",
      "interviewing",
      "offer",
      "rejected",
    ]);
  });

  it("NAV_TABS contains 5 main views with keys", () => {
    expect(NAV_TABS).toHaveLength(5);
    const ids = NAV_TABS.map((t) => t.id);
    expect(ids).toEqual(["discovered", "tracked", "metrics", "scraper", "settings"]);
  });

  it("ROLE_TYPES matches ROLE_TYPE_LABELS mapping", () => {
    for (const role of ROLE_TYPES) {
      expect(ROLE_TYPE_LABELS[role]).toBeDefined();
      expect(typeof ROLE_TYPE_LABELS[role]).toBe("string");
    }
  });

  it("CRON_PRESETS are populated and valid", () => {
    expect(CRON_PRESETS.length).toBeGreaterThan(0);
    for (const preset of CRON_PRESETS) {
      expect(preset.label).toBeTruthy();
      expect(preset.expr.split(" ").length).toBeGreaterThanOrEqual(5);
    }
  });
});
