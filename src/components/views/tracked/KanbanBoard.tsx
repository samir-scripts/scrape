import { useState, useEffect, useRef } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { TRACKED_STAGES } from "../../../lib/constants";
import { useKanbanDrag } from "./useKanbanDrag";
import { KanbanColumn } from "./KanbanColumn";

interface KanbanBoardProps {
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onDropJob: (jobId: string, targetStatus: JobStatus) => void;
  pendingIds?: ReadonlySet<string>;
  onDelete?: (job: Job) => void;
}

export function KanbanBoard({
  jobs,
  selectedJobId,
  onSelectJob,
  onDropJob,
  onDelete,
  pendingIds,
}: KanbanBoardProps) {
  const [stage, setStage] = useState<JobStatus>("applied");
  const scroll = useRef<HTMLDivElement>(null);
  const drag = useKanbanDrag(jobs, onDropJob, scroll);

  // Auto-scroll selected card into view without shifting document scroll
  useEffect(() => {
    if (selectedJobId) {
      const el = document.getElementById(`kanban-card-${selectedJobId}`);
      el?.scrollIntoView({ block: "nearest", behavior: "auto" });
    }
  }, [selectedJobId]);

  return (
    <div className="flex-1 min-h-0 flex flex-col bg-[var(--color-paper-2)]">
      <span id="kanban-drag-instructions" className="sr-only">Drag the handle to a stage. Or press Space, left/right arrows, Enter to confirm, Escape to cancel.</span>
      <p role="status" className="sr-only">{drag.announcement}</p>
      <div className="px-4 py-2 flex gap-4 items-center">
        <p className="text-xs text-[var(--color-ink-mute)]">
          Showing applications on this page. Stage counts describe loaded jobs.
        </p>
        <label className="kanban-stage-picker text-sm">
          Stage{" "}
          <select
            aria-label="Board stage"
            value={stage}
            onChange={(e) => { setStage(e.target.value as JobStatus); document.getElementById(`kanban-stage-${e.target.value}`)?.scrollIntoView({ block: "nearest", inline: "start" }); }}
            className="border p-2 bg-[var(--color-paper)]"
          >
            {TRACKED_STAGES.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div ref={scroll} className="tracked-scroll flex-1 min-h-0 overflow-x-auto overflow-y-hidden">
        <div className="kanban-columns flex h-full gap-4 p-4 w-max min-w-full">
          {TRACKED_STAGES.map((value, i) => (
            <div
              key={value}
              className={`kanban-column w-[300px] shrink-0 min-h-0 ${stage === value ? "kanban-active" : ""}`}
            >
              <KanbanColumn
                status={value}
                columnIndex={i}
                jobs={jobs.filter((job) => job.status === value)}
                selectedJobId={selectedJobId}
                draggingJobId={drag.current?.active ? drag.current.job.id : null}
                dropTarget={drag.current?.active ? drag.current.target : null}
                handleProps={drag.handlers}
                pendingIds={pendingIds}
                onSelectJob={onSelectJob}
                onDropJob={onDropJob}
                onDelete={onDelete}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
