import type { Source } from "../bindings/Source";

export type WorkflowDisplayStatus =
  | "RUNNING"
  | "PAUSED"
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
 * 2. PAUSED: workflow toggle disabled (muted gray outline)
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
      colorClass: "text-[var(--color-signal)]",
      bulletClass: "text-[var(--color-signal)] animate-pulse",
      bulletGlyph: "■",
      isPulsing: true,
    };
  }

  if (!source.enabled) {
    return {
      label: "PAUSED",
      colorClass: "text-[var(--color-ink-mute)]",
      bulletClass: "text-[var(--color-ink-mute)]",
      bulletGlyph: "□",
      isPulsing: false,
    };
  }

  if (source.last_status === "error") {
    return {
      label: "FAILED",
      colorClass: "text-[var(--color-cherry)]",
      bulletClass: "text-[var(--color-cherry)]",
      bulletGlyph: "■",
      isPulsing: false,
    };
  }

  if (source.last_status === "ok" || source.last_status === "success") {
    return {
      label: "SUCCESS",
      colorClass: "text-[var(--color-lime)]",
      bulletClass: "text-[var(--color-lime)]",
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
