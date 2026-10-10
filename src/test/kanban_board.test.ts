import { describe, it, expect } from "vitest";
import {
  buildColumnMatrix,
  findJobCoordinates,
  navigateVertical,
  navigateHorizontal,
  shiftJobStage,
} from "../lib/kanbanNavigation";
import { TRACKED_STAGES, STATUS_CONFIG } from "../lib/constants";
import type { Job } from "../bindings/Job";

const createMockJob = (id: string, status: any): Job =>
  ({
    id,
    dedupe_key: `dedupe-${id}`,
    source_id: null,
    external_id: `ext-${id}`,
    status,
    title: `Title ${id}`,
    company: `Company ${id}`,
    role_type: "full_time",
    location: "Remote",
    url: `https://example.com/job/${id}`,
    date_posted: null,
    date_added: "2026-10-10T00:00:00Z",
    last_seen_at: "2026-10-10T00:00:00Z",
    status_changed_at: "2026-10-10T00:00:00Z",
    notes: null,
    favorited: false,
    favorited_at: null,
    salary_min: 120000,
    salary_max: 160000,
    salary_currency: "USD",
    salary_interval: "year",
    salary_raw: "$120k-$160k",
    salary_source: "structured",
    workplace_type: "remote",
    sponsorship: "unknown",
    sponsorship_evidence: null,
    clearance: "unknown",
    clearance_level: null,
    clearance_evidence: null,
    experience_min: null,
    experience_max: null,
    experience_raw: null,
    is_new_grad: "unknown",
    experience_evidence: null,
    description_text: null,
    enrichment_version: 1,
    locations: [],
  } as Job);

describe("Kanban 2D Matrix Navigation & Stage Shifting", () => {
  const jobs: Job[] = [
    createMockJob("app-1", "applied"),
    createMockJob("app-2", "applied"),
    createMockJob("int-1", "interviewing"),
    createMockJob("off-1", "offer"),
    createMockJob("rej-1", "rejected"),
  ];

  it("builds 4 discrete columns strictly adhering to TRACKED_STAGES", () => {
    const matrix = buildColumnMatrix(jobs);
    expect(matrix).toHaveLength(4);
    expect(matrix.map((c) => c.stage)).toEqual(TRACKED_STAGES);
    expect(matrix[0].jobs).toHaveLength(2); // APPLIED
    expect(matrix[1].jobs).toHaveLength(1); // INTERVIEWING
    expect(matrix[2].jobs).toHaveLength(1); // OFFER
    expect(matrix[3].jobs).toHaveLength(1); // REJECTED
  });

  it("locates job coordinates accurately in 2D space", () => {
    const matrix = buildColumnMatrix(jobs);
    expect(findJobCoordinates(matrix, "app-1")).toEqual({ colIdx: 0, rowIdx: 0 });
    expect(findJobCoordinates(matrix, "app-2")).toEqual({ colIdx: 0, rowIdx: 1 });
    expect(findJobCoordinates(matrix, "int-1")).toEqual({ colIdx: 1, rowIdx: 0 });
    expect(findJobCoordinates(matrix, "off-1")).toEqual({ colIdx: 2, rowIdx: 0 });
    expect(findJobCoordinates(matrix, "rej-1")).toEqual({ colIdx: 3, rowIdx: 0 });
    expect(findJobCoordinates(matrix, "non-existent")).toBeNull();
    expect(findJobCoordinates(matrix, null)).toBeNull();
  });

  it("navigates vertically within the same column without jumping across columns", () => {
    const matrix = buildColumnMatrix(jobs);
    // Down from app-1 to app-2 in Column 0
    expect(navigateVertical(matrix, "app-1", 1)).toBe("app-2");
    // Clamps to bottom of Column 0
    expect(navigateVertical(matrix, "app-2", 1)).toBe("app-2");
    // Up from app-2 to app-1
    expect(navigateVertical(matrix, "app-2", -1)).toBe("app-1");
    // Clamps to top of Column 0
    expect(navigateVertical(matrix, "app-1", -1)).toBe("app-1");
  });

  it("navigates horizontally across columns maintaining closest row index", () => {
    const matrix = buildColumnMatrix(jobs);
    // From app-1 (col 0, row 0) to interviewing (col 1, row 0)
    expect(navigateHorizontal(matrix, "app-1", 1)).toBe("int-1");
    // From app-2 (col 0, row 1) to interviewing (col 1, row 0, clamped)
    expect(navigateHorizontal(matrix, "app-2", 1)).toBe("int-1");
    // From off-1 (col 2) back to int-1 (col 1)
    expect(navigateHorizontal(matrix, "off-1", -1)).toBe("int-1");
    // Left at col 0 does not wrap or crash
    expect(navigateHorizontal(matrix, "app-1", -1)).toBe("app-1");
    // Right at col 3 does not wrap or crash
    expect(navigateHorizontal(matrix, "rej-1", 1)).toBe("rej-1");
  });

  it("skips empty columns gracefully during horizontal navigation", () => {
    const sparseJobs: Job[] = [
      createMockJob("app-1", "applied"),
      createMockJob("off-1", "offer"),
    ];
    const matrix = buildColumnMatrix(sparseJobs);
    // Navigate right from APPLIED skips empty INTERVIEWING straight to OFFER
    expect(navigateHorizontal(matrix, "app-1", 1)).toBe("off-1");
    // Navigate left from OFFER skips empty INTERVIEWING straight to APPLIED
    expect(navigateHorizontal(matrix, "off-1", -1)).toBe("app-1");
  });

  it("advances and rewinds pipeline stages sequentially without wrapping", () => {
    expect(shiftJobStage("applied", 1)).toBe("interviewing");
    expect(shiftJobStage("interviewing", 1)).toBe("offer");
    expect(shiftJobStage("offer", 1)).toBe("rejected");
    expect(shiftJobStage("rejected", 1)).toBeNull();

    expect(shiftJobStage("rejected", -1)).toBe("offer");
    expect(shiftJobStage("offer", -1)).toBe("interviewing");
    expect(shiftJobStage("interviewing", -1)).toBe("applied");
    expect(shiftJobStage("applied", -1)).toBeNull();
  });

  it("handles empty matrix fallback gracefully", () => {
    const emptyMatrix = buildColumnMatrix([]);
    expect(navigateVertical(emptyMatrix, null, 1)).toBeNull();
    expect(navigateHorizontal(emptyMatrix, null, 1)).toBeNull();
    expect(navigateHorizontal(emptyMatrix, null, -1)).toBeNull();
  });

  it("handles orphaned or null jobId with directional boundary fallbacks", () => {
    const matrix = buildColumnMatrix(jobs);
    // Moving right from null/orphaned picks the first non-empty column from the left
    expect(navigateHorizontal(matrix, null, 1)).toBe("app-1");
    expect(navigateHorizontal(matrix, "orphaned-discovered-job-id", 1)).toBe("app-1");

    // Moving left from null/orphaned picks the last non-empty column from the right
    expect(navigateHorizontal(matrix, null, -1)).toBe("rej-1");
    expect(navigateHorizontal(matrix, "orphaned-discovered-job-id", -1)).toBe("rej-1");

    // Vertical fallback
    expect(navigateVertical(matrix, null, 1)).toBe("app-1");
    expect(navigateVertical(matrix, null, -1)).toBe("app-2");
  });

  it("preserves vertical row memory across unequal column heights", () => {
    const matrix = buildColumnMatrix(jobs);
    // Col 0 has 2 jobs (app-1 @ row 0, app-2 @ row 1)
    // Col 1 has 1 job (int-1 @ row 0)
    // Navigating from app-2 (row 1) to Col 1 clamps to int-1 (row 0), but with preferredRowIdx=1
    const nextColJob = navigateHorizontal(matrix, "app-2", 1, 1);
    expect(nextColJob).toBe("int-1");

    // Moving back left with preferredRowIdx=1 symmetrically lands on app-2 (row 1)
    const returnColJob = navigateHorizontal(matrix, nextColJob, -1, 1);
    expect(returnColJob).toBe("app-2");
  });

  it("STATUS_CONFIG maps colors and labels for all tracked stages", () => {
    for (const stage of TRACKED_STAGES) {
      const cfg = STATUS_CONFIG[stage];
      expect(cfg).toBeDefined();
      expect(cfg.label).toBe(stage.toUpperCase());
      expect(cfg.bg).toBeTruthy();
      expect(cfg.border).toBeTruthy();
      expect(cfg.text).toBeTruthy();
    }
  });
});

