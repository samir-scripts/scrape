import { useMemo } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { useUIStore } from "../../../lib/store";
import { TRACKED_STAGES } from "../../../lib/constants";
import { KanbanBoard } from "./KanbanBoard";
import { TrackedList } from "./TrackedList";
import { JobDetailDrawer } from "./JobDetailDrawer";
import { useHotkeys } from "../../../hooks/useHotkeys";

interface TrackedViewProps {
  jobs: Job[];
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
  onUndo: () => void;
}

export function TrackedView({
  jobs,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onUndo,
}: TrackedViewProps) {
  const {
    trackedViewMode,
    selectedJobId,
    setSelectedJobId,
    drawerJobId,
    setDrawerJobId,
    searchQuery,
  } = useUIStore();

  const filteredJobs = useMemo(() => {
    return jobs.filter((j) => {
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesTitle = j.title.toLowerCase().includes(q);
        const matchesCompany = j.company.toLowerCase().includes(q);
        const matchesLocation = j.location?.toLowerCase().includes(q) ?? false;
        if (!matchesTitle && !matchesCompany && !matchesLocation) {
          return false;
        }
      }
      return true;
    });
  }, [jobs, searchQuery]);

  const selectedJob = useMemo(() => {
    return filteredJobs.find((j) => j.id === selectedJobId) || null;
  }, [filteredJobs, selectedJobId]);

  const drawerJob = useMemo(() => {
    return filteredJobs.find((j) => j.id === drawerJobId) || null;
  }, [filteredJobs, drawerJobId]);

  const currentIndex = useMemo(() => {
    return filteredJobs.findIndex((j) => j.id === selectedJobId);
  }, [filteredJobs, selectedJobId]);

  // Stage shift helper: move job to prev/next stage in TRACKED_STAGES
  const shiftStage = (direction: -1 | 1) => {
    if (!selectedJob) return;
    const currentStageIdx = TRACKED_STAGES.indexOf(selectedJob.status);
    if (currentStageIdx === -1) return;
    const nextStageIdx = currentStageIdx + direction;
    if (nextStageIdx >= 0 && nextStageIdx < TRACKED_STAGES.length) {
      onUpdateStatus(selectedJob.id, TRACKED_STAGES[nextStageIdx]);
    }
  };

  useHotkeys({
    onNextItem: () => {
      if (filteredJobs.length === 0) return;
      const nextIdx = Math.min(currentIndex + 1, filteredJobs.length - 1);
      setSelectedJobId(filteredJobs[nextIdx].id);
    },
    onPrevItem: () => {
      if (filteredJobs.length === 0) return;
      const prevIdx = Math.max(currentIndex - 1, 0);
      setSelectedJobId(filteredJobs[prevIdx].id);
    },
    onOpenUrl: () => {
      if (selectedJob) onOpenUrl(selectedJob.url);
    },
    onShiftLeft: () => shiftStage(-1),
    onShiftRight: () => shiftStage(1),
    onUndo,
  });

  const handleSelectJob = (job: Job) => {
    setSelectedJobId(job.id);
    setDrawerJobId(job.id);
  };

  return (
    <div className="flex-1 w-full h-full relative overflow-hidden flex flex-col bg-[var(--color-paper)]">
      {trackedViewMode === "kanban" ? (
        <KanbanBoard
          jobs={filteredJobs}
          selectedJobId={selectedJobId}
          onSelectJob={handleSelectJob}
          onDropJob={onUpdateStatus}
        />
      ) : (
        <TrackedList
          jobs={filteredJobs}
          selectedJobId={selectedJobId}
          onSelectJob={handleSelectJob}
          onOpenUrl={onOpenUrl}
        />
      )}

      {/* Slide-out detail drawer */}
      <JobDetailDrawer
        job={drawerJob}
        isOpen={drawerJobId !== null}
        onClose={() => setDrawerJobId(null)}
        onUpdateStatus={onUpdateStatus}
        onUpdateNotes={onUpdateNotes}
        onOpenUrl={onOpenUrl}
      />
    </div>
  );
}
