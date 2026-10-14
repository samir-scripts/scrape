import { RemoveJobButton } from "../../ui/RemoveJobButton";
import { useState, useRef, type DragEvent } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { STATUS_CONFIG } from "../../../lib/constants";
import { formatSalary } from "../../../lib/filter";
import { formatJobLocation } from "../../../lib/jobLocationLabel";

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
      onDragEndJob();
    }
  };

  return (
    <section
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      aria-label={`${config.label} stage`}
      className={`flex flex-col h-full overflow-hidden bg-[var(--color-paper-2)] ${isOver ? "outline outline-2 outline-[var(--color-focus)] outline-offset-[-2px]" : ""}`}
    >
      <header className="min-h-12 shrink-0 px-4 py-3 border-b flex items-center justify-between gap-2">
        <h3 className="font-bold text-sm normal-case">
          {config.label.toLowerCase()}
        </h3>
        <span className="text-xs">{jobs.length} loaded</span>
      </header>
      <div className="flex-1 overflow-y-auto p-3 flex flex-col gap-3">
        {!jobs.length ? (
          <p className="text-sm text-[var(--color-ink-mute)] p-3">
            No jobs loaded in this stage. Check other pages or move an
            application here.
          </p>
        ) : (
          jobs.map((job) => (
            <article
              key={job.id}
              id={`kanban-card-${job.id}`}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData("text/plain", job.id);
                e.dataTransfer.effectAllowed = "move";
                onDragStartJob(job.id);
              }}
              onDragEnd={onDragEndJob}
              className={`border p-4 grid gap-3 select-text bg-[var(--color-paper)] ${job.id === selectedJobId ? "outline outline-2 outline-[var(--color-focus)] outline-offset-[-2px]" : "border-[var(--color-ink-mute)]"} ${job.id === draggingJobId ? "border-dashed" : ""}`}
            >
              <div className="flex items-start justify-between gap-2">
                <button
                  type="button"
                  aria-label={`View ${job.title} at ${job.company}`}
                  className="normal-case text-left font-bold text-base leading-snug cursor-pointer line-clamp-2 underline"
                  title={job.title}
                  onClick={() => onSelectJob(job)}
                >
                  {job.title}
                </button>
                <span
                  aria-hidden="true"
                  title="Drag to another stage"
                  className="cursor-grab text-[var(--color-ink-mute)]"
                >
                  ⠿
                </span>
              </div>
              <p className="text-sm">{job.company}</p>
              <p className="text-sm text-[var(--color-ink-mute)]">
                {formatJobLocation(job.location, job.workplace_type, job.locations)}
              </p>
              <p className="text-sm">{formatSalary(job)}</p>
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs text-[var(--color-ink-mute)]">
                  {[
                    job.favorited ? "Favorited" : "",
                    job.notes?.trim() ? "Has notes" : "",
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
                {onDelete && (
                  <RemoveJobButton
                    title={job.title}
                    company={job.company}
                    onClick={() => onDelete(job)}
                  />
                )}
              </div>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
