import { useEffect, useMemo } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { useUIStore } from "../../../lib/store";
import { ThreeColumn } from "../../layout/ThreeColumn";
import { ListRow } from "../../ui/ListRow";
import { Tag } from "../../ui/Tag";
import { EmptyState } from "../../ui/EmptyState";
import { JobDetail } from "./JobDetail";
import { JobMetaPanel } from "./JobMetaPanel";
import { useHotkeys } from "../../../hooks/useHotkeys";
import { formatExperience, formatSalary } from "../../../lib/filter";

interface DiscoveredViewProps {
  jobs: Job[];
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
  onUndo: () => void;
  onToggleFavorite?: (jobId: string) => void;
}

export function DiscoveredView({
  jobs,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onUndo,
  onToggleFavorite,
}: DiscoveredViewProps) {
  const {
    filterSpec,
    selectedJobId,
    setSelectedJobId,
  } = useUIStore();

  const filteredJobs = jobs;

  // Keep selection valid or pick the first job
  useEffect(() => {
    if (filteredJobs.length > 0) {
      const exists = filteredJobs.some((j) => j.id === selectedJobId);
      if (!exists) {
        setSelectedJobId(filteredJobs[0].id);
      }
    } else {
      setSelectedJobId(null);
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
    onUndo,
  });

  return (
    <ThreeColumn
      left={
        filteredJobs.length === 0 ? (
          <div className="p-4">
            <EmptyState
              title="NO POSTINGS FOUND"
              description={
                filterSpec.search
                  ? "TRY CLEARING FILTERS OR SEARCH TERMS."
                  : "ALL DISCOVERED JOBS HAVE BEEN TRIAGED."
              }
            />
          </div>
        ) : (
          <div className="flex flex-col w-full divide-y-0">
            {filteredJobs.map((job) => {
              const isSelected = job.id === selectedJobId;
              const expText = formatExperience(job);
              const salText = formatSalary(job);

              return (
                <ListRow
                  key={job.id}
                  selected={isSelected}
                  onClick={() => setSelectedJobId(job.id)}
                >
                  <div className="flex flex-col gap-1 pr-1">
                    <div className="flex items-center justify-between gap-1">
                      <div className="flex items-center gap-1 min-w-0">
                        {onToggleFavorite && (
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              onToggleFavorite(job.id);
                            }}
                            className={`px-0.5 text-xs font-mono cursor-pointer transition-colors ${
                              job.favorited
                                ? "text-[var(--color-lime)] font-black"
                                : "text-[var(--color-ink-mute)] hover:text-[var(--color-ink)]"
                            }`}
                            title={job.favorited ? "Favorited [S]" : "Favorite [S]"}
                            aria-label={job.favorited ? "Unfavorite posting" : "Favorite posting"}
                          >
                            {job.favorited ? "★" : "☆"}
                          </button>
                        )}
                        <span className="font-bold truncate max-w-[170px]">
                          {job.title}
                        </span>
                      </div>
                      <Tag
                        status={job.status}
                        className={isSelected ? "border-[var(--color-paper)]" : ""}
                      />
                    </div>
                    <div className="flex items-center justify-between text-[11px] opacity-80 font-mono">
                      <span className="truncate max-w-[140px]">
                        {job.company}
                      </span>
                      <span className="truncate max-w-[100px] text-right">
                        {job.location || "REMOTE"}
                      </span>
                    </div>

                    {/* Enrichment Badges Row */}
                    <div className="flex items-center flex-wrap gap-1 text-[9px] font-mono mt-0.5">
                      {salText !== "—" && (
                        <span className="px-1 border border-current opacity-90 font-bold">
                          {salText}
                        </span>
                      )}
                      {expText !== "—" && (
                        <span className="px-1 border border-current opacity-80">
                          {expText}
                        </span>
                      )}
                      {job.is_new_grad === "yes" && (
                        <span className="px-1 bg-[var(--color-ink)] text-[var(--color-paper)] font-bold">
                          GRAD
                        </span>
                      )}
                      {job.sponsorship === "yes" && (
                        <span className="px-1 border border-current opacity-70">
                          SPON:Y
                        </span>
                      )}
                      {job.sponsorship === "no" && (
                        <span className="px-1 border border-current opacity-50">
                          SPON:N
                        </span>
                      )}
                      {job.clearance === "yes" && (
                        <span className="px-1 border border-[var(--color-cherry)] text-[var(--color-cherry)] font-bold">
                          CLR:REQ
                        </span>
                      )}
                    </div>
                  </div>
                </ListRow>
              );
            })}
          </div>
        )
      }
      center={
        <JobDetail
          job={selectedJob}
          onUpdateNotes={onUpdateNotes}
          onOpenUrl={onOpenUrl}
        />
      }
      right={
        <JobMetaPanel
          job={selectedJob}
          onUpdateStatus={onUpdateStatus}
          onOpenUrl={onOpenUrl}
          onToggleFavorite={onToggleFavorite}
        />
      }
    />
  );
}
