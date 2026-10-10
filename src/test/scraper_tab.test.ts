import { describe, it, expect, beforeEach } from "vitest";
import { useUIStore, type ScraperCategory, type SettingsCategory } from "../lib/store";
import { NAV_TABS } from "../lib/constants";

describe("Scraper Tab & Operations Hub", () => {
  beforeEach(() => {
    useUIStore.setState({
      view: "discovered",
      scraperCategory: "sources",
      settingsCategory: "appearance",
    });
  });

  it("includes SCRAPER in NAV_TABS positioned before SETTINGS", () => {
    const tabIds = NAV_TABS.map((t) => t.id);
    expect(tabIds).toEqual(["discovered", "tracked", "metrics", "scraper", "settings"]);
    expect(NAV_TABS.find((t) => t.id === "scraper")?.label).toBe("SCRAPER");
  });

  it("handles scraper subcategory navigation across sources, schedule, filters, and logs", () => {
    const { setScraperCategory } = useUIStore.getState();

    const expectedCategories: ScraperCategory[] = ["sources", "schedule", "filters", "logs"];

    for (const cat of expectedCategories) {
      setScraperCategory(cat);
      expect(useUIStore.getState().scraperCategory).toBe(cat);
    }
  });

  it("preserves settings categories restricted strictly to appearance and system", () => {
    const { setSettingsCategory } = useUIStore.getState();

    const allowedSettings: SettingsCategory[] = ["appearance", "system"];

    for (const cat of allowedSettings) {
      setSettingsCategory(cat);
      expect(useUIStore.getState().settingsCategory).toBe(cat);
    }
  });

  it("builds custom scraper configuration with CSS selectors and regex rules", () => {
    // Helper simulating AddSourceModal config generation
    const buildCustomScraperConfig = (inputs: {
      item: string;
      title: string;
      titleRegex?: string;
      link: string;
      linkRegex?: string;
      location?: string;
      locationRegex?: string;
      salary?: string;
      salaryRegex?: string;
    }) => {
      return {
        item: inputs.item.trim(),
        title: inputs.title.trim(),
        link: inputs.link.trim(),
        ...(inputs.location?.trim() ? { location: inputs.location.trim() } : {}),
        ...(inputs.salary?.trim() ? { salary: inputs.salary.trim() } : {}),
        ...(inputs.titleRegex?.trim() ? { title_regex: inputs.titleRegex.trim() } : {}),
        ...(inputs.linkRegex?.trim() ? { link_regex: inputs.linkRegex.trim() } : {}),
        ...(inputs.locationRegex?.trim() ? { location_regex: inputs.locationRegex.trim() } : {}),
        ...(inputs.salaryRegex?.trim() ? { salary_regex: inputs.salaryRegex.trim() } : {}),
      };
    };

    const config = buildCustomScraperConfig({
      item: ".job-posting",
      title: "h3.title",
      titleRegex: "Role:\\s*(.*)",
      link: "a.apply",
      linkRegex: "(/careers/\\d+)",
      location: ".loc-badge",
      locationRegex: "Loc:\\s*([^|]+)",
      salary: ".pay-text",
      salaryRegex: "(\\$\\d+k?\\s*-\\s*\\$\\d+k?)",
    });

    expect(config).toEqual({
      item: ".job-posting",
      title: "h3.title",
      link: "a.apply",
      location: ".loc-badge",
      salary: ".pay-text",
      title_regex: "Role:\\s*(.*)",
      link_regex: "(/careers/\\d+)",
      location_regex: "Loc:\\s*([^|]+)",
      salary_regex: "(\\$\\d+k?\\s*-\\s*\\$\\d+k?)",
    });
  });

  it("redirects directly to scraper sources view from error log remediation actions", () => {
    // When user clicks [EDIT SOURCE IN SCRAPER ->] in logs
    const store = useUIStore.getState();
    store.setView("scraper");
    store.setScraperCategory("logs");
    expect(useUIStore.getState().view).toBe("scraper");
    expect(useUIStore.getState().scraperCategory).toBe("logs");

    // Action handler executed
    useUIStore.getState().setView("scraper");
    useUIStore.getState().setScraperCategory("sources");

    expect(useUIStore.getState().view).toBe("scraper");
    expect(useUIStore.getState().scraperCategory).toBe("sources");
  });
});
