import { describe, it, expect, beforeEach } from "vitest";
import {
  useUIStore,
  DEFAULT_FILTER_SPEC,
  type SettingsCategory,
  type ScraperCategory,
} from "../lib/store";

describe("UI Store - Filters & Logs State", () => {
  beforeEach(() => {
    useUIStore.getState().resetFilterSpec();
    useUIStore.getState().setFilterPanelOpen(false);
  });

  it("initializes with default filter spec", () => {
    const { filterSpec } = useUIStore.getState();
    expect(filterSpec).toEqual(DEFAULT_FILTER_SPEC);
    expect(filterSpec.salary_min).toBeNull();
    expect(filterSpec.experience_min).toBeNull();
    expect(filterSpec.new_grad).toBe("any");
  });

  it("updates filter spec with partial parameters and function updaters", () => {
    const store = useUIStore.getState();

    // Partial object update
    store.setFilterSpec({ salary_min: 120000, salary_max: 180000 });
    expect(useUIStore.getState().filterSpec.salary_min).toBe(120000);
    expect(useUIStore.getState().filterSpec.salary_max).toBe(180000);

    // Functional updater
    store.setFilterSpec((prev) => ({
      ...prev,
      experience_min: 3,
      new_grad: "only",
    }));
    const updated = useUIStore.getState().filterSpec;
    expect(updated.salary_min).toBe(120000);
    expect(updated.experience_min).toBe(3);
    expect(updated.new_grad).toBe("only");
  });

  it("resets filter spec to defaults", () => {
    const store = useUIStore.getState();
    store.setFilterSpec({
      salary_min: 150000,
      sponsorship: ["yes"],
      countries: ["US"],
    });

    store.resetFilterSpec();
    expect(useUIStore.getState().filterSpec).toEqual(DEFAULT_FILTER_SPEC);
  });

  it("toggles and sets filter panel visibility", () => {
    const store = useUIStore.getState();
    expect(store.isFilterPanelOpen).toBe(false);

    store.toggleFilterPanel();
    expect(useUIStore.getState().isFilterPanelOpen).toBe(true);

    store.toggleFilterPanel();
    expect(useUIStore.getState().isFilterPanelOpen).toBe(false);

    store.setFilterPanelOpen(true);
    expect(useUIStore.getState().isFilterPanelOpen).toBe(true);
  });

  it("supports scraper categories (sources, schedule, filters, logs)", () => {
    const categories: ScraperCategory[] = [
      "sources",
      "schedule",
      "filters",
      "logs",
    ];
    expect(categories).toContain("logs");

    useUIStore.getState().setScraperCategory("logs");
    expect(useUIStore.getState().scraperCategory).toBe("logs");

    useUIStore.getState().setScraperCategory("sources");
    expect(useUIStore.getState().scraperCategory).toBe("sources");
  });

  it("supports streamlined settings categories (appearance, system)", () => {
    const categories: SettingsCategory[] = [
      "appearance",
      "system",
    ];
    expect(categories).toEqual(["appearance", "system"]);

    useUIStore.getState().setSettingsCategory("system");
    expect(useUIStore.getState().settingsCategory).toBe("system");
  });
});
