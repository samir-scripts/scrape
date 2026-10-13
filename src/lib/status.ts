import type { Source } from "../bindings/Source";

export type WorkflowDisplayStatus =
  | "RUNNING"
  | "DISABLED"
  | "FAILED"
  | "SUCCESS"
  | "UNTESTED";

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
  source: Pick<Source, "enabled" | "last_status">,
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
