import { describe, it, expect, beforeEach } from "vitest";
import { getWorkflowStatus } from "../lib/status";
import { useUIStore } from "../lib/store";
import type { ScrapeLogEntry } from "../bindings/ScrapeLogEntry";

describe("Workflow Status Engine (getWorkflowStatus)", () => {
  it("prioritizes RUNNING when isRunning is true, even if source was failed or disabled", () => {
    const disabledAndFailed = {
      enabled: false,
      last_status: "error",
    };
    const status = getWorkflowStatus(disabledAndFailed, true);
    expect(status.label).toBe("RUNNING");
    expect(status.colorClass).toContain("color-signal");
    expect(status.bulletClass).toContain("animate-pulse");
    expect(status.isPulsing).toBe(true);
  });

  it("returns PAUSED when workflow is disabled and not running", () => {
    const disabledSource = {
      enabled: false,
      last_status: "ok",
    };
    const status = getWorkflowStatus(disabledSource, false);
    expect(status.label).toBe("PAUSED");
    expect(status.bulletGlyph).toBe("□");
    expect(status.colorClass).toContain("color-ink-mute");
    expect(status.isPulsing).toBe(false);
  });

  it("returns FAILED when last_status is error", () => {
    const failedSource = {
      enabled: true,
      last_status: "error",
    };
    const status = getWorkflowStatus(failedSource, false);
    expect(status.label).toBe("FAILED");
    expect(status.bulletGlyph).toBe("■");
    expect(status.colorClass).toContain("color-cherry");
    expect(status.isPulsing).toBe(false);
  });

  it("returns SUCCESS when last_status is ok or success", () => {
    const okSource = {
      enabled: true,
      last_status: "ok",
    };
    const statusOk = getWorkflowStatus(okSource, false);
    expect(statusOk.label).toBe("SUCCESS");
    expect(statusOk.colorClass).toContain("color-lime");

    const successSource = {
      enabled: true,
      last_status: "success",
    };
    const statusSuccess = getWorkflowStatus(successSource, false);
    expect(statusSuccess.label).toBe("SUCCESS");
    expect(statusSuccess.colorClass).toContain("color-lime");
  });

  it("returns UNTESTED when last_status is null or untested", () => {
    const freshSource = {
      enabled: true,
      last_status: null,
    };
    const statusFresh = getWorkflowStatus(freshSource, false);
    expect(statusFresh.label).toBe("UNTESTED");
    expect(statusFresh.colorClass).toContain("color-ink-mute");

    const untestedSource = {
      enabled: true,
      last_status: "untested",
    };
    const statusUntested = getWorkflowStatus(untestedSource, false);
    expect(statusUntested.label).toBe("UNTESTED");
  });
});

describe("UIStore runningSourceIds Lifecycle", () => {
  beforeEach(() => {
    useUIStore.getState().clearRunningSources();
  });

  it("tracks and clears running source IDs dynamically", () => {
    const { setSourceRunning, clearRunningSources } = useUIStore.getState();

    expect(useUIStore.getState().runningSourceIds).toEqual([]);

    // Add source 1
    setSourceRunning("source-cloudflare", true);
    expect(useUIStore.getState().runningSourceIds).toContain("source-cloudflare");

    // Add source 2
    setSourceRunning("source-stripe", true);
    expect(useUIStore.getState().runningSourceIds).toEqual([
      "source-cloudflare",
      "source-stripe",
    ]);

    // Duplicate add is idempotent
    setSourceRunning("source-cloudflare", true);
    expect(useUIStore.getState().runningSourceIds).toEqual([
      "source-cloudflare",
      "source-stripe",
    ]);

    // Remove source 1
    setSourceRunning("source-cloudflare", false);
    expect(useUIStore.getState().runningSourceIds).toEqual(["source-stripe"]);

    // Clear all
    clearRunningSources();
    expect(useUIStore.getState().runningSourceIds).toEqual([]);
  });
});

describe("Execution Log Filtering Logic", () => {
  const sampleLogs: ScrapeLogEntry[] = [
    {
      id: "log-1",
      run_id: "run-100",
      timestamp: "2026-10-11T12:00:00Z",
      level: "info",
      source_id: "src-cloudflare",
      message: "Scrape workflow initiated: Cloudflare (greenhouse)",
    },
    {
      id: "log-2",
      run_id: "run-100",
      timestamp: "2026-10-11T12:00:01Z",
      level: "info",
      source_id: "src-cloudflare",
      message: "Fetched 124 postings, matched 15 jobs",
    },
    {
      id: "log-3",
      run_id: "run-100",
      timestamp: "2026-10-11T12:00:02Z",
      level: "warn",
      source_id: "src-cloudflare",
      message: "Rate limit threshold near 80%",
    },
    {
      id: "log-4",
      run_id: "run-100",
      timestamp: "2026-10-11T12:00:03Z",
      level: "error",
      source_id: "src-cloudflare",
      message: "Failed to parse salary structure for job 9912",
    },
    {
      id: "log-5",
      run_id: "run-100",
      timestamp: "2026-10-11T12:00:04Z",
      level: "info",
      source_id: null,
      message: "Execution finished: committed 15 new jobs",
    },
  ];

  it("filters logs for 'all' mode preserving unscoped and scoped logs", () => {
    const inspectedSourceId = "src-cloudflare";
    const scoped = sampleLogs.filter(
      (l) => !l.source_id || l.source_id === inspectedSourceId
    );
    expect(scoped.length).toBe(5);
  });

  it("filters logs for 'successes' mode (info only)", () => {
    const inspectedSourceId = "src-cloudflare";
    const successes = sampleLogs
      .filter((l) => !l.source_id || l.source_id === inspectedSourceId)
      .filter((l) => l.level === "info");

    expect(successes.length).toBe(3);
    expect(successes.every((l) => l.level === "info")).toBe(true);
  });

  it("filters logs for 'errors' mode (error and warn only)", () => {
    const inspectedSourceId = "src-cloudflare";
    const errors = sampleLogs
      .filter((l) => !l.source_id || l.source_id === inspectedSourceId)
      .filter((l) => l.level === "error" || l.level === "warn");

    expect(errors.length).toBe(2);
    expect(errors.map((l) => l.id)).toEqual(["log-3", "log-4"]);
  });
});
