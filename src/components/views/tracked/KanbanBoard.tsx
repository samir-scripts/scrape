import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { KanbanColumn } from "./KanbanColumn";
import { TRACKED_STAGES } from "../../../lib/constants";

interface KanbanBoardProps {
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onDropJob: (jobId: string, targetStatus: JobStatus) => void;
}

const STAGE_TITLES: Record<string, string> = {
  applied: "APPLIED",
  interviewing: "INTERVIEWING",
  offer: "OFFER",
  rejected: "REJECTED",
};

export function KanbanBoard({
  jobs,
  selectedJobId,
  onSelectJob,
  onDropJob,
}: KanbanBoardProps) {
  return (
    <div className="flex-1 flex w-full h-full overflow-x-auto bg-[var(--color-paper)]">
      {TRACKED_STAGES.map((stage) => {
        const stageJobs = jobs.filter((j) => j.status === stage);
        return (
          <KanbanColumn
            key={stage}
            status={stage}
            title={STAGE_TITLES[stage] || stage}
            jobs={stageJobs}
            selectedJobId={selectedJobId}
            onSelectJob={onSelectJob}
            onDropJob={onDropJob}
          />
        );
      })}
    </div>
  );
}
