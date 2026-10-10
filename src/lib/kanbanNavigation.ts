import { TRACKED_STAGES } from "./constants";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";

export interface ColumnMatrix {
  stage: JobStatus;
  jobs: Job[];
}

/**
  * Builds a 4-column matrix strictly matching TRACKED_STAGES (applied -> interviewing -> offer -> rejected).
  */
export function buildColumnMatrix(jobs: Job[]): ColumnMatrix[] {
  return TRACKED_STAGES.map((stage) => ({
    stage,
    jobs: jobs.filter((j) => j.status === stage),
  }));
}

/**
  * Finds the 2D column and row indices of a given job ID within the matrix.
  */
export function findJobCoordinates(
  matrix: ColumnMatrix[],
  jobId: string | null
): { colIdx: number; rowIdx: number } | null {
  if (!jobId) return null;
  for (let c = 0; c < matrix.length; c++) {
    const r = matrix[c].jobs.findIndex((j) => j.id === jobId);
    if (r !== -1) return { colIdx: c, rowIdx: r };
  }
  return null;
}

/**
  * Navigates vertically (up or down) within the currently selected job's column.
  * Clamps to column top/bottom boundaries without wrapping to other columns.
  */
export function navigateVertical(
  matrix: ColumnMatrix[],
  currentJobId: string | null,
  delta: 1 | -1
): string | null {
  const coords = findJobCoordinates(matrix, currentJobId);
  if (!coords) {
    const firstNonEmpty = matrix.find((c) => c.jobs.length > 0);
    return firstNonEmpty?.jobs[0]?.id ?? null;
  }

  const colJobs = matrix[coords.colIdx].jobs;
  if (colJobs.length === 0) return null;

  const targetRow = Math.max(0, Math.min(colJobs.length - 1, coords.rowIdx + delta));
  return colJobs[targetRow]?.id ?? currentJobId;
}

/**
  * Navigates horizontally (left or right) to adjacent non-empty columns.
  * Retains the closest vertical row index in the destination column.
  */
export function navigateHorizontal(
  matrix: ColumnMatrix[],
  currentJobId: string | null,
  delta: 1 | -1
): string | null {
  const coords = findJobCoordinates(matrix, currentJobId);
  if (!coords) {
    const firstNonEmpty = matrix.find((c) => c.jobs.length > 0);
    return firstNonEmpty?.jobs[0]?.id ?? null;
  }

  let nextCol = coords.colIdx + delta;
  while (nextCol >= 0 && nextCol < matrix.length) {
    const targetJobs = matrix[nextCol].jobs;
    if (targetJobs.length > 0) {
      const targetRow = Math.min(coords.rowIdx, targetJobs.length - 1);
      return targetJobs[targetRow].id;
    }
    nextCol += delta;
  }

  return currentJobId;
}

/**
  * Shifts an application stage left or right along TRACKED_STAGES.
  * Returns the new JobStatus or null if the boundary has been reached.
  */
export function shiftJobStage(
  currentStatus: JobStatus,
  delta: 1 | -1
): JobStatus | null {
  const idx = TRACKED_STAGES.indexOf(currentStatus);
  if (idx === -1) return null;
  const nextIdx = idx + delta;
  if (nextIdx >= 0 && nextIdx < TRACKED_STAGES.length) {
    return TRACKED_STAGES[nextIdx];
  }
  return null;
}
