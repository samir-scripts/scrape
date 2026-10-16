import { ResizableSplitPane } from "../../layout/ResizableSplitPane";
import { useMemo, useEffect, useLayoutEffect, useRef } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { useUIStore } from "../../../lib/store";
import {
  buildColumnMatrix,
  findJobCoordinates,
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
  onUpdateStatus: (jobId: string, status: JobStatus) => void | Promise<unknown>;
  onUpdateNotes: (jobId: string, notes: string) => void | Promise<unknown>;
  onOpenUrl: (url: string) => void;
  pendingIds?: ReadonlySet<string>;
  onUndo: () => void;
  onToggleFavorite?: (jobId: string) => void;
  onRequestDelete?: (job: Job) => void;
}

export function TrackedView({
  jobs,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onUndo,
  pendingIds,
  onToggleFavorite,
  onRequestDelete,
}: TrackedViewProps) {
  const {
    trackedViewMode,
    setFilterSpec,
    filterSpec,
    selectedJobId,
    setSelectedJobId,
    drawerJobId,
    setDrawerJobId,
  } = useUIStore();

  const preferredRowRef = useRef<number>(0);
  const contentRef = useRef<HTMLDivElement>(null);
  const scrollPositions = useRef<Record<string, {top:number;left:number}>>({});
  useLayoutEffect(() => {
    const element = contentRef.current?.querySelector<HTMLElement>(".tracked-scroll");
    const saved = scrollPositions.current[trackedViewMode];
    if (element && saved) { element.scrollTop = saved.top; element.scrollLeft = saved.left; }
  }, [trackedViewMode]);

  const filteredJobs = jobs;

  // Keep selection valid within TrackedView or pick the first tracked job
  useEffect(() => {
    if (filteredJobs.length > 0) {
      const exists = filteredJobs.some((j) => j.id === selectedJobId);
      if (!exists) {
        setSelectedJobId(filteredJobs[0].id);
        preferredRowRef.current = 0;
      }
    } else {
      setSelectedJobId(null);
      preferredRowRef.current = 0;
    }
  }, [filteredJobs, selectedJobId, setSelectedJobId]);

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
        if (nextId) {
          setSelectedJobId(nextId);
          const coords = findJobCoordinates(columnMatrix, nextId);
          if (coords) preferredRowRef.current = coords.rowIdx;
        }
      } else {
        if (filteredJobs.length === 0) return;
        const nextIdx = Math.min(currentIndex + 1, filteredJobs.length - 1);
        setSelectedJobId(filteredJobs[nextIdx].id);
      }
    },
    onPrevItem: () => {
      if (trackedViewMode === "kanban") {
        const prevId = navigateVertical(columnMatrix, selectedJobId, -1);
        if (prevId) {
          setSelectedJobId(prevId);
          const coords = findJobCoordinates(columnMatrix, prevId);
          if (coords) preferredRowRef.current = coords.rowIdx;
        }
      } else {
        if (filteredJobs.length === 0) return;
        const prevIdx = Math.max(currentIndex - 1, 0);
        setSelectedJobId(filteredJobs[prevIdx].id);
      }
    },
    onColLeft: () => {
      if (trackedViewMode === "kanban") {
        const nextId = navigateHorizontal(
          columnMatrix,
          selectedJobId,
          -1,
          preferredRowRef.current
        );
        if (nextId) setSelectedJobId(nextId);
      }
    },
    onColRight: () => {
      if (trackedViewMode === "kanban") {
        const nextId = navigateHorizontal(
          columnMatrix,
          selectedJobId,
          1,
          preferredRowRef.current
        );
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
    onToggleFavorite: () => {
      if (selectedJob && onToggleFavorite) {
        onToggleFavorite(selectedJob.id);
      }
    },
    onDelete: () => {
      if (selectedJob && onRequestDelete) {
        onRequestDelete(selectedJob);
      }
    },
    onUndo,
  });

  const handleSelectJob = (job: Job) => {
    setSelectedJobId(job.id);
    setDrawerJobId(job.id);
    const coords = findJobCoordinates(columnMatrix, job.id);
    if (coords) preferredRowRef.current = coords.rowIdx;
  };

  return <ResizableSplitPane primary={
      <div ref={contentRef} onScrollCapture={event => { const element = event.target as HTMLElement; if (element.classList.contains("tracked-scroll")) scrollPositions.current[trackedViewMode] = {top:element.scrollTop,left:element.scrollLeft}; }} className="flex-1 min-w-0 min-h-0 flex flex-col">
      {trackedViewMode === "kanban" ? (
        <KanbanBoard
          pendingIds={pendingIds}
          jobs={filteredJobs}
          selectedJobId={selectedJobId}
          onSelectJob={handleSelectJob}
          onDropJob={onUpdateStatus}
          onDelete={onRequestDelete}
        />
      ) : (
        <TrackedList
          repositorySort={filterSpec.sort || "status_changed_at_desc"}
          onRepositorySort={(sort) => setFilterSpec({ sort })}
          jobs={filteredJobs}
          selectedJobId={selectedJobId}
          onSelectJob={handleSelectJob}
          onToggleFavorite={onToggleFavorite}
          onDelete={onRequestDelete}
        />
      )}

      </div>

  } secondary={drawerJob ? ({ inline, reset }) => (
      <JobDetailDrawer
        job={drawerJob}
        mode={inline ? "inline" : "modal"}
        onResetPane={reset}
        isOpen={drawerJobId !== null}
        onClose={() => setDrawerJobId(null)}
        onUpdateStatus={onUpdateStatus}
        onUpdateNotes={onUpdateNotes}
        onOpenUrl={onOpenUrl}
        onToggleFavorite={onToggleFavorite}
        onDelete={onRequestDelete}
      />
  ) : undefined} />;
}
