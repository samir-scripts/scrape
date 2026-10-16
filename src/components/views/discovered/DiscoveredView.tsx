import { ResizableSplitPane, type SplitLayout } from "../../layout/ResizableSplitPane";
import { countActiveFilters } from "../../../lib/filter";
import { useEffect, useMemo } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { useUIStore } from "../../../lib/store";
import { JobCard } from "./JobCard";
import { JobDetail } from "./JobDetail";
import { EmptyState } from "../../ui/EmptyState";
import { useHotkeys } from "../../../hooks/useHotkeys";

interface DiscoveredViewProps {
  jobs: Job[];
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void | Promise<unknown>;
  onOpenUrl: (url: string) => void;
  onUndo: () => void;
  onToggleFavorite?: (jobId: string) => void;
  onRequestDelete?: (job: Job) => void;
}

export function DiscoveredView({
  jobs,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onUndo,
  onToggleFavorite,
  onRequestDelete,
}: DiscoveredViewProps) {
  const {
    filterSpec,
    selectedJobId,
    setSelectedJobId,
  } = useUIStore();

  const filteredJobs = jobs;

  // Clear selection only if currently selected job was removed or filtered out
  useEffect(() => {
    if (selectedJobId !== null) {
      const exists = filteredJobs.some((j) => j.id === selectedJobId);
      if (!exists) {
        setSelectedJobId(null);
      }
    }
  }, [filteredJobs, selectedJobId, setSelectedJobId]);

  const selectedJob = useMemo(() => {
    return filteredJobs.find((j) => j.id === selectedJobId) || null;
  }, [filteredJobs, selectedJobId]);

  // Auto-mark viewed after 1 second of selection
  useEffect(() => {
    if (!selectedJob || selectedJob.status !== "scraped") return;

    const timer = setTimeout(() => {
      onUpdateStatus(selectedJob.id, "viewed");
    }, 1000);

    return () => clearTimeout(timer);
  }, [selectedJob?.id, selectedJob?.status, onUpdateStatus]);

  // Keyboard navigation
  const currentIndex = useMemo(() => {
    return filteredJobs.findIndex((j) => j.id === selectedJobId);
  }, [filteredJobs, selectedJobId]);

  useHotkeys({
    onNextItem: () => {
      if (filteredJobs.length === 0) return;
      if (selectedJobId === null) {
        setSelectedJobId(filteredJobs[0].id);
      } else {
        const nextIdx = Math.min(currentIndex + 1, filteredJobs.length - 1);
        setSelectedJobId(filteredJobs[nextIdx].id);
      }
    },
    onPrevItem: () => {
      if (filteredJobs.length === 0) return;
      if (selectedJobId === null) {
        setSelectedJobId(filteredJobs[0].id);
      } else {
        const prevIdx = Math.max(currentIndex - 1, 0);
        setSelectedJobId(filteredJobs[prevIdx].id);
      }
    },
    onOpenUrl: () => {
      if (selectedJob) onOpenUrl(selectedJob.url);
    },
    onApply: () => {
      if (selectedJob) onUpdateStatus(selectedJob.id, "applied");
    },
    onIgnore: () => {
      if (selectedJob) onUpdateStatus(selectedJob.id, "ignored");
    },
    onViewed: () => {
      if (selectedJob && selectedJob.status === "scraped") {
        onUpdateStatus(selectedJob.id, "viewed");
      }
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

  const leftContent =
    filteredJobs.length === 0 ? (
      <div className="p-8 flex items-center justify-center w-full">
        <EmptyState
          title="NO POSTINGS FOUND"
          description={
            countActiveFilters(filterSpec) > 0
              ? "No postings match the current filters. Clear filters or change the matching rules."
              : "No saved discoveries available. Run a workflow or review your tracked jobs."
          }
        />
      </div>
    ) : (
      <div className="flex flex-col gap-3 p-4 w-full">
        {filteredJobs.map((job) => (
          <JobCard
            key={job.id}
            job={job}
            selected={job.id === selectedJobId}
            onClick={() => setSelectedJobId(job.id)}
            onToggleFavorite={onToggleFavorite}
            onDelete={onRequestDelete}
          />
        ))}
      </div>
    );

  const rightContent = selectedJob ? ({ inline, reset }: SplitLayout) => (
    <JobDetail
      job={selectedJob}
      mode={inline ? "inline" : "modal"}
      onResetPane={reset}
      onClose={() => setSelectedJobId(null)}
      onUpdateStatus={onUpdateStatus}
      onUpdateNotes={onUpdateNotes}
      onOpenUrl={onOpenUrl}
      onToggleFavorite={onToggleFavorite}
      onDelete={onRequestDelete}
    />
  ) : undefined;

  return <ResizableSplitPane primary={<div className="flex-1 min-w-0 overflow-y-auto"><div className="max-w-[1000px] mx-auto">{leftContent}</div></div>} secondary={rightContent} />;
}
