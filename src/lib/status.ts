import type { RunStatus } from "../bindings/RunStatus";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import type { Source } from "../bindings/Source";

export type WorkflowDisplayStatus =
  | "RUNNING"
  | "DISABLED"
  | "FAILED"
  | "SUCCESS"
  | "SUCCESS WITH WARNINGS"
  | "PARTIAL"
  | "CANCELLED"
  | "UNTESTED"
  | "NEEDS REVIEW";

export interface WorkflowStatusVisual {
  label: WorkflowDisplayStatus;
  colorClass: string;
  bulletClass: string;
  bulletGlyph: string;
  isPulsing: boolean;
}

/**
 * Resolves the display status and NieR brutalist visual styling for a workflow source.
 * Priority hierarchy:
 * 1. RUNNING: active execution in progress (signal blue pulse)
 * 2. DISABLED: workflow toggle disabled (muted gray outline)
 * 3. FAILED: last run produced an error (cherry red solid)
 * 4. SUCCESS: last run completed cleanly (lime green solid)
 * 5. UNTESTED: no recorded run yet (muted gray solid)
 */
export function getWorkflowStatus(
  source: Pick<Source, "enabled" | "last_status" | "last_outcome">,
  isRunning: boolean
): WorkflowStatusVisual {
  if (isRunning) {
    return {
      label: "RUNNING",
      colorClass: "text-[var(--color-information)]",
      bulletClass: "text-[var(--color-information)] animate-pulse",
      bulletGlyph: "■",
      isPulsing: true,
    };
  }

  if (!source.enabled) {
    return {
      label: "DISABLED",
      colorClass: "text-[var(--color-ink-mute)]",
      bulletClass: "text-[var(--color-ink-mute)]",
      bulletGlyph: "□",
      isPulsing: false,
    };
  }

  if (source.last_outcome === "success_with_warnings" || source.last_outcome === "partial" || source.last_outcome === "cancelled") {
    const label = runStatusLabel(source.last_outcome) as WorkflowDisplayStatus;
    const colorClass = source.last_outcome === "cancelled" ? "text-[var(--color-ink-mute)]" : "text-[var(--color-warning)]";
    return { label, colorClass, bulletClass: colorClass, bulletGlyph: "■", isPulsing: false };
  }
  if (source.last_status === "error") {
    return {
      label: "FAILED",
      colorClass: "text-[var(--color-danger)]",
      bulletClass: "text-[var(--color-danger)]",
      bulletGlyph: "■",
      isPulsing: false,
    };
  }

  if (source.last_status === "ok" || source.last_status === "success") {
    return {
      label: "SUCCESS",
      colorClass: "text-[var(--color-success)]",
      bulletClass: "text-[var(--color-success)]",
      bulletGlyph: "■",
      isPulsing: false,
    };
  }

  return {
    label: "UNTESTED",
    colorClass: "text-[var(--color-ink-mute)]",
    bulletClass: "text-[var(--color-ink-mute)]",
    bulletGlyph: "■",
    isPulsing: false,
  };
}

export function sourceNeedsAttention(source: Pick<Source, "last_status" | "last_outcome">): boolean {
  return source.last_outcome ? ["failed", "partial"].includes(source.last_outcome) : source.last_status === "error";
}
export function runRequiresAttention(run: Pick<ScrapeRun, "status" | "error">): boolean {
  return run.status === "failed" || run.status === "partial" || (!["cancelled", "success_with_warnings"].includes(run.status) && !!run.error);
}
export function runStatusLabel(status: RunStatus): string { return status.replaceAll("_", " ").toUpperCase(); }
export function runStatusBadge(status: RunStatus): string {
  if (status === "success") return "bg-[var(--color-lime)] text-[var(--color-accent-ink)] border-[var(--color-ink)]";
  if (status === "success_with_warnings" || status === "partial") return "bg-[var(--color-paper)] text-[var(--color-warning)] border-[var(--color-warning)]";
  if (status === "cancelled") return "bg-[var(--color-paper-2)] text-[var(--color-ink-mute)] border-[var(--color-ink-mute)]";
  if (status === "running") return "bg-[var(--color-paper-2)] text-[var(--color-ink)] border-[var(--color-ink)] animate-pulse";
  return "bg-[var(--color-danger)] text-[var(--color-paper)] border-[var(--color-danger)]";
}
