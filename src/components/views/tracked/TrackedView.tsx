import { useMemo } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { useUIStore } from "../../../lib/store";
import {
  buildColumnMatrix,
  navigateVertical,
  navigateHorizontal,
  shiftJobStage,
} from "../../../lib/kanbanNavigation";
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

  const columnMatrix = useMemo(() => {
    return buildColumnMatrix(filteredJobs);
  }, [filteredJobs]);

  useHotkeys({
    onNextItem: () => {
      if (trackedViewMode === "kanban") {
        const nextId = navigateVertical(columnMatrix, selectedJobId, 1);
        if (nextId) setSelectedJobId(nextId);
      } else {
        if (filteredJobs.length === 0) return;
        const nextIdx = Math.min(currentIndex + 1, filteredJobs.length - 1);
        setSelectedJobId(filteredJobs[nextIdx].id);
      }
    },
    onPrevItem: () => {
      if (trackedViewMode === "kanban") {
        const prevId = navigateVertical(columnMatrix, selectedJobId, -1);
        if (prevId) setSelectedJobId(prevId);
      } else {
        if (filteredJobs.length === 0) return;
        const prevIdx = Math.max(currentIndex - 1, 0);
        setSelectedJobId(filteredJobs[prevIdx].id);
      }
    },
    onColLeft: () => {
      if (trackedViewMode === "kanban") {
        const nextId = navigateHorizontal(columnMatrix, selectedJobId, -1);
        if (nextId) setSelectedJobId(nextId);
      }
    },
    onColRight: () => {
      if (trackedViewMode === "kanban") {
        const nextId = navigateHorizontal(columnMatrix, selectedJobId, 1);
        if (nextId) setSelectedJobId(nextId);
      }
    },
    onOpenUrl: () => {
      if (selectedJob) onOpenUrl(selectedJob.url);
    },
    onShiftLeft: () => {
      if (!selectedJob) return;
      const nextStatus = shiftJobStage(selectedJob.status, -1);
      if (nextStatus) onUpdateStatus(selectedJob.id, nextStatus);
    },
    onShiftRight: () => {
      if (!selectedJob) return;
      const nextStatus = shiftJobStage(selectedJob.status, 1);
      if (nextStatus) onUpdateStatus(selectedJob.id, nextStatus);
    },
    onUndo,
  });

  const handleSelectJob = (job: Job) => {
    setSelectedJobId(job.id);
    setDrawerJobId(job.id);
  };

  return (
    <div
      className={`flex-1 w-full h-full relative overflow-hidden flex flex-col bg-[var(--color-paper)] transition-[padding] duration-150 ${
        drawerJobId ? "pr-96" : ""
      }`}
    >
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
