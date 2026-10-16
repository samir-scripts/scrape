import { describe, it, expect } from "vitest";
import { runRequiresAttention } from "../lib/status";
import type { ScrapeRun } from "../bindings/ScrapeRun";

const createMockRun = (overrides: Partial<ScrapeRun> = {}): ScrapeRun => ({
  id: "run-1",
  trigger: "manual",
  status: "success",
  started_at: "2026-10-09T10:00:00Z",
  finished_at: "2026-10-09T10:01:00Z",
  jobs_found: BigInt(50),
  jobs_new: BigInt(10),
  error: null,
  source_id: null,
  ...overrides,
});

describe("Scraper Logs & Run History", () => {
  it("sorts runs chronologically with newest first (started_at DESC)", () => {
    const run1 = createMockRun({ id: "run-old", started_at: "2026-10-01T12:00:00Z" });
    const run2 = createMockRun({ id: "run-newest", started_at: "2026-10-09T14:30:00Z" });
    const run3 = createMockRun({ id: "run-mid", started_at: "2026-10-05T09:15:00Z" });

    const runs = [run1, run2, run3];
    const sorted = [...runs].sort((a, b) => {
      return new Date(b.started_at).getTime() - new Date(a.started_at).getTime();
    });

    expect(sorted.map((r) => r.id)).toEqual(["run-newest", "run-mid", "run-old"]);
  });

  it("identifies runs requiring attention with red square error marker", () => {
    const isRunError = runRequiresAttention;

    const successRun = createMockRun({ status: "success", error: null });
    const failedRun = createMockRun({ status: "failed", error: "Connection timed out" });
    const partialRun = createMockRun({ status: "partial", error: null });
    const erroredSuccessRun = createMockRun({ status: "success", error: "Source 2 HTTP 500" });

    expect(isRunError(successRun)).toBe(false);
    expect(isRunError(failedRun)).toBe(true);
    expect(isRunError(partialRun)).toBe(true);
    expect(isRunError(erroredSuccessRun)).toBe(true);
    expect(isRunError(createMockRun({status: "cancelled", error: "Explicit cancellation"}))).toBe(false);
    expect(isRunError(createMockRun({status: "success_with_warnings", error: "Optional enrichment warning"}))).toBe(false);
  });

  it("filters runs between all runs and errors only", () => {
    const isRunError = runRequiresAttention;

    const runs = [
      createMockRun({ id: "1", status: "success" }),
      createMockRun({ id: "2", status: "failed", error: "Crash" }),
      createMockRun({ id: "3", status: "partial" }),
      createMockRun({ id: "4", status: "success" }),
    ];

    const errorRuns = runs.filter(isRunError);
    expect(errorRuns).toHaveLength(2);
    expect(errorRuns.map((r) => r.id)).toEqual(["2", "3"]);
  });
});
