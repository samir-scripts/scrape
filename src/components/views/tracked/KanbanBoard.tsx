import { useState, useEffect } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { TRACKED_STAGES } from "../../../lib/constants";
import { KanbanColumn } from "./KanbanColumn";

interface KanbanBoardProps {
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onDropJob: (jobId: string, targetStatus: JobStatus) => void;
}

export function KanbanBoard({
  jobs,
  selectedJobId,
  onSelectJob,
  onDropJob,
}: KanbanBoardProps) {
  const [draggingJobId, setDraggingJobId] = useState<string | null>(null);

  // Auto-scroll selected card into view without shifting document scroll
  useEffect(() => {
    if (selectedJobId) {
      const el = document.getElementById(`kanban-card-${selectedJobId}`);
      el?.scrollIntoView({ block: "nearest", behavior: "auto" });
    }
  }, [selectedJobId]);

  return (
    <div className="flex-1 w-full h-full overflow-x-auto overflow-y-hidden bg-[var(--color-paper)]">
      <div className="grid grid-cols-4 min-w-[1120px] w-full h-full divide-x divide-[var(--color-ink)]">
        {TRACKED_STAGES.map((stage, colIdx) => {
          const stageJobs = jobs.filter((j) => j.status === stage);
          return (
            <KanbanColumn
              key={stage}
              status={stage}
              columnIndex={colIdx}
              jobs={stageJobs}
              selectedJobId={selectedJobId}
              draggingJobId={draggingJobId}
              onSelectJob={onSelectJob}
              onDropJob={onDropJob}
              onDragStartJob={(id) => setDraggingJobId(id)}
              onDragEndJob={() => setDraggingJobId(null)}
            />
          );
        })}
      </div>
    </div>
  );
}
