import { describe, it, expect, beforeEach } from "vitest";
import { useUIStore } from "../lib/store";

describe("useUIStore state transitions", () => {
  beforeEach(() => {
    // Reset store state between tests
    useUIStore.setState({
      view: "discovered",
      selectedJobId: null,
      selectedJobIds: [],
      trackedViewMode: "kanban",
      drawerJobId: null,
      scraperCategory: "sources",
      settingsCategory: "appearance",
      isScanning: false,
      scanStatusText: "",
      undoStack: [],
    });
    useUIStore.getState().resetFilterSpec();
  });

  it("handles view navigation and localStorage persistence", () => {
    const { setView } = useUIStore.getState();
    expect(useUIStore.getState().view).toBe("discovered");

    setView("tracked");
    expect(useUIStore.getState().view).toBe("tracked");

    setView("metrics");
    expect(useUIStore.getState().view).toBe("metrics");

    setView("scraper");
    expect(useUIStore.getState().view).toBe("scraper");

    setView("settings");
    expect(useUIStore.getState().view).toBe("settings");
  });

  it("manages single selection and filter changes", () => {
    const { setSelectedJobId, setFilterSpec } = useUIStore.getState();

    setSelectedJobId("job-123");
    expect(useUIStore.getState().selectedJobId).toBe("job-123");

    setFilterSpec({ role_types: ["full_time"], search: "Staff Engineer" });
    expect(useUIStore.getState().filterSpec.role_types).toEqual(["full_time"]);
    expect(useUIStore.getState().filterSpec.search).toBe("Staff Engineer");
  });

  it("manages multi-selection and clearing", () => {
    const { toggleJobSelection, clearJobSelection } = useUIStore.getState();
    expect(useUIStore.getState().selectedJobIds).toEqual([]);

    toggleJobSelection("job-1");
    expect(useUIStore.getState().selectedJobIds).toEqual(["job-1"]);

    toggleJobSelection("job-2");
    expect(useUIStore.getState().selectedJobIds).toEqual(["job-1", "job-2"]);

    // Toggling job-1 again should unselect it
    toggleJobSelection("job-1");
    expect(useUIStore.getState().selectedJobIds).toEqual(["job-2"]);

    toggleJobSelection("job-3");
    clearJobSelection();
    expect(useUIStore.getState().selectedJobIds).toEqual([]);
  });

  it("switches tracked view mode between kanban and list", () => {
    const { setTrackedViewMode } = useUIStore.getState();
    expect(useUIStore.getState().trackedViewMode).toBe("kanban");

    setTrackedViewMode("list");
    expect(useUIStore.getState().trackedViewMode).toBe("list");

    setTrackedViewMode("kanban");
    expect(useUIStore.getState().trackedViewMode).toBe("kanban");
  });

  it("manages scanning status and progress text", () => {
    const { setScanning } = useUIStore.getState();
    expect(useUIStore.getState().isScanning).toBe(false);

    setScanning(true, "Scraping greenhouse...");
    expect(useUIStore.getState().isScanning).toBe(true);
    expect(useUIStore.getState().scanStatusText).toBe("Scraping greenhouse...");

    setScanning(false);
    expect(useUIStore.getState().isScanning).toBe(false);
    expect(useUIStore.getState().scanStatusText).toBe("");
  });

  it("handles LIFO undo stack operations and max capacity capping", () => {
    const { pushUndo, popUndo } = useUIStore.getState();
    expect(popUndo()).toBeUndefined();

    pushUndo({ jobId: "job-1", previousStatus: "scraped" });
    pushUndo({ jobId: "job-2", previousStatus: "viewed" });

    expect(useUIStore.getState().undoStack).toHaveLength(2);

    // Should pop in LIFO order
    const top1 = popUndo();
    expect(top1).toEqual({ jobId: "job-2", previousStatus: "viewed" });

    const top2 = popUndo();
    expect(top2).toEqual({ jobId: "job-1", previousStatus: "scraped" });

    expect(popUndo()).toBeUndefined();

    // Verify capping at 20 items
    for (let i = 0; i < 25; i++) {
      pushUndo({ jobId: `job-${i}`, previousStatus: "scraped" });
    }
    expect(useUIStore.getState().undoStack.length).toBeLessThanOrEqual(20);
  });

  it("manages scraperMode state transitions between simple and advanced", () => {
    const { setScraperMode } = useUIStore.getState();
    expect(useUIStore.getState().scraperMode).toBe("simple");

    setScraperMode("advanced");
    expect(useUIStore.getState().scraperMode).toBe("advanced");

    setScraperMode("simple");
    expect(useUIStore.getState().scraperMode).toBe("simple");
  });
});
