import { useState, useRef, type DragEvent } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { STATUS_CONFIG } from "../../../lib/constants";
import { Tag } from "../../ui/Tag";
import { CompanyLogo } from "../../ui/CompanyLogo";

interface KanbanColumnProps {
  status: JobStatus;
  columnIndex: number;
  jobs: Job[];
  selectedJobId: string | null;
  draggingJobId: string | null;
  onSelectJob: (job: Job) => void;
  onDropJob: (jobId: string, targetStatus: JobStatus) => void;
  onDragStartJob: (jobId: string) => void;
  onDragEndJob: () => void;
  onDelete?: (job: Job) => void;
}

export function KanbanColumn({
  status,
  columnIndex,
  jobs,
  selectedJobId,
  draggingJobId,
  onSelectJob,
  onDropJob,
  onDragStartJob,
  onDragEndJob,
  onDelete,
}: KanbanColumnProps) {
  const [isOver, setIsOver] = useState(false);
  const dragCounterRef = useRef(0);
  const config = STATUS_CONFIG[status] || {
    label: status.toUpperCase(),
    bg: "var(--color-ink)",
    text: "var(--color-paper)",
    border: "var(--color-ink)",
  };

  const handleDragEnter = (e: DragEvent) => {
    e.preventDefault();
    dragCounterRef.current += 1;
    if (e.dataTransfer.types.includes("text/plain")) {
      setIsOver(true);
    }
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  };

  const handleDragLeave = () => {
    dragCounterRef.current -= 1;
    if (dragCounterRef.current <= 0) {
      setIsOver(false);
      dragCounterRef.current = 0;
    }
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    setIsOver(false);
    dragCounterRef.current = 0;
    const jobId = e.dataTransfer.getData("text/plain");
    if (jobId) {
      onDropJob(jobId, status);
    }
  };

  return (
    <div
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`flex flex-col h-full overflow-hidden bg-[var(--color-paper)] transition-colors select-none ${
        isOver ? "bg-[var(--color-paper-2)]" : ""
      }`}
    >
      {/* Pinned Brutalist Header (h-9) */}
      <div className="h-9 shrink-0 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {/* Stage index & status accent pip */}
          <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
            // 0{columnIndex + 1}
          </span>
          <span
            className="w-2 h-2 border border-[var(--color-ink)] shrink-0"
            style={{ backgroundColor: config.bg }}
            aria-hidden="true"
          />
          <h3 className="font-mono font-bold text-xs uppercase tracking-wider text-[var(--color-ink)]">
            {config.label}
          </h3>
        </div>

        {/* Tabular Count Badge */}
        <span className="font-mono text-[10px] font-bold px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] min-w-[28px] text-center">
          {String(jobs.length).padStart(2, "0")}
        </span>
      </div>

      {/* Cards List with Stable Scrollbar Gutter */}
      <div className="flex-1 overflow-y-scroll p-2.5 flex flex-col gap-2">
        {jobs.length === 0 ? (
          <div
            className={`flex-1 min-h-[140px] border border-dashed border-[var(--color-ink-mute)] flex flex-col items-center justify-center p-3 text-center transition-colors ${
              isOver
                ? "border-solid border-[var(--color-ink)] bg-[var(--color-paper-2)] font-bold"
                : "bg-[var(--color-paper-2)]/20"
            }`}
          >
            <span className="text-[10px] font-mono font-bold tracking-wider text-[var(--color-ink-mute)]">
              [ + ] DROP TO STAGE
            </span>
            <span className="text-[9px] font-mono text-[var(--color-ink-mute)]/60 mt-1">
              // NO ACTIVE APPLICATIONS
            </span>
          </div>
        ) : (
          jobs.map((job) => {
            const isSelected = job.id === selectedJobId;
            const isDragging = job.id === draggingJobId;

            return (
              <div
                key={job.id}
                id={`kanban-card-${job.id}`}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("text/plain", job.id);
                  e.dataTransfer.effectAllowed = "move";
                  setTimeout(() => onDragStartJob(job.id), 0);
                }}
                onDragEnd={onDragEndJob}
                onClick={() => onSelectJob(job)}
                className={`p-3 border border-[var(--color-ink)] cursor-pointer select-none transition-none flex flex-col gap-1.5 ${
                  isDragging
                    ? "opacity-30 border-dashed"
                    : isSelected
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <div className="flex items-center justify-between gap-1">
                  <div className="flex items-center gap-1.5 min-w-0">
                    {job.favorited && (
                      <svg
                        className="w-3.5 h-3.5 text-[var(--color-lime)] shrink-0"
                        viewBox="0 0 24 24"
                        fill="var(--color-lime)"
                        stroke="currentColor"
                        strokeWidth="1.5"
                      >
                        <polygon points="12,2 15.09,8.26 22,9.27 17,14.14 18.18,21.02 12,17.77 5.82,21.02 7,14.14 2,9.27 8.91,8.26" />
                      </svg>
                    )}
                    <CompanyLogo
                      company={job.company}
                      iconUrl={job.company_icon_url}
                      size="sm"
                    />
                    <span className="font-bold text-xs truncate max-w-[150px]">
                      {job.company}
                    </span>
                  </div>
                  <Tag
                    label={job.role_type.toUpperCase()}
                    variant={isSelected ? "outline" : "muted"}
                    className={
                      isSelected
                        ? "border-[var(--color-paper)] text-[var(--color-paper)]"
                        : ""
                    }
                  />
                </div>

                <div className="text-xs truncate">{job.title}</div>

                <div className="flex items-center justify-between text-[10px] font-mono opacity-80 pt-1 border-t border-current">
                  <span className="truncate max-w-[120px]">{job.location || "REMOTE"}</span>
                  <div className="flex items-center gap-1.5 shrink-0">
                    {job.notes && (
                      <span className="font-bold truncate max-w-[80px]">
                        NOTE ▏
                      </span>
                    )}
                    {onDelete && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onDelete(job);
                        }}
                        title="Delete application record"
                        className="opacity-60 hover:opacity-100 hover:text-[var(--color-cherry)] text-[9px] font-bold px-1 py-0 border border-current transition-opacity cursor-pointer"
                      >
                        DEL
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
