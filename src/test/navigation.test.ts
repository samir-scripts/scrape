import { describe, it, expect } from "vitest";
import { NAV_TABS, TRACKED_STAGES } from "../lib/constants";
import type { NavTabId } from "../lib/constants";
import type { JobStatus } from "../bindings/JobStatus";

describe("Navigation & Hotkey Helpers", () => {
  function getNextTab(current: NavTabId): NavTabId {
    const currentIdx = NAV_TABS.findIndex((t) => t.id === current);
    const nextIdx = (currentIdx + 1) % NAV_TABS.length;
    return NAV_TABS[nextIdx].id;
  }

  function getPrevTab(current: NavTabId): NavTabId {
    const currentIdx = NAV_TABS.findIndex((t) => t.id === current);
    const prevIdx = (currentIdx - 1 + NAV_TABS.length) % NAV_TABS.length;
    return NAV_TABS[prevIdx].id;
  }

  function getNextTrackedStage(current: JobStatus): JobStatus | null {
    const idx = TRACKED_STAGES.indexOf(current);
    if (idx < 0 || idx >= TRACKED_STAGES.length - 1) return null;
    return TRACKED_STAGES[idx + 1];
  }

  function getPrevTrackedStage(current: JobStatus): JobStatus | null {
    const idx = TRACKED_STAGES.indexOf(current);
    if (idx <= 0) return null;
    return TRACKED_STAGES[idx - 1];
  }

  it("cycles forward through tabs with wrap-around", () => {
    expect(getNextTab("discovered")).toBe("tracked");
    expect(getNextTab("tracked")).toBe("metrics");
    expect(getNextTab("metrics")).toBe("scraper");
    expect(getNextTab("scraper")).toBe("settings");
    expect(getNextTab("settings")).toBe("discovered");
  });

  it("cycles backward through tabs with wrap-around", () => {
    expect(getPrevTab("discovered")).toBe("settings");
    expect(getPrevTab("settings")).toBe("scraper");
    expect(getPrevTab("scraper")).toBe("metrics");
    expect(getPrevTab("metrics")).toBe("tracked");
    expect(getPrevTab("tracked")).toBe("discovered");
  });

  it("advances and rewinds tracked pipeline stages correctly without wrapping", () => {
    expect(getNextTrackedStage("applied")).toBe("interviewing");
    expect(getNextTrackedStage("interviewing")).toBe("offer");
    expect(getNextTrackedStage("offer")).toBe("rejected");
    expect(getNextTrackedStage("rejected")).toBeNull();

    expect(getPrevTrackedStage("rejected")).toBe("offer");
    expect(getPrevTrackedStage("offer")).toBe("interviewing");
    expect(getPrevTrackedStage("interviewing")).toBe("applied");
    expect(getPrevTrackedStage("applied")).toBeNull();
  });
});
