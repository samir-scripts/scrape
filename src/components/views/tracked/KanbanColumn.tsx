import { RemoveJobButton } from "../../ui/RemoveJobButton";
import { IconButton } from "../../ui/IconButton";
import { Icon } from "../../ui/Icon";
import type { ButtonHTMLAttributes } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { STATUS_CONFIG, TRACKED_STAGES } from "../../../lib/constants";
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
  dropTarget: JobStatus | null;
  handleProps: (job: Job) => ButtonHTMLAttributes<HTMLButtonElement>;
  pendingIds?: ReadonlySet<string>;
  onDelete?: (job: Job) => void;
}

export function KanbanColumn({
  status,
  jobs,
  selectedJobId,
  draggingJobId,
  onSelectJob,
  onDropJob,
  dropTarget,
  handleProps,
  pendingIds,
  onDelete,
}: KanbanColumnProps) {
  const isOver = dropTarget === status;
  const config = STATUS_CONFIG[status] || {
    label: status.toUpperCase(),
    bg: "var(--color-ink)",
    text: "var(--color-paper)",
    border: "var(--color-ink)",
  };

  return (
    <section
      id={`kanban-stage-${status}`}
      data-kanban-stage={status}
      aria-label={`${config.label} stage`}
      className={`flex flex-col h-full overflow-hidden border border-[var(--color-ink-mute)] bg-[var(--color-paper-2)] ${isOver ? "outline outline-2 outline-[var(--color-focus)] outline-offset-[-2px]" : ""}`}
    >
      <header className="min-h-12 shrink-0 px-4 py-3 border-b flex items-center justify-between gap-2">
        <h3 className="font-bold text-sm normal-case">
          {config.label.toLowerCase()}
        </h3>
        <span className="text-xs">{jobs.length} loaded{isOver ? " · Drop here" : ""}</span>
      </header>
      <div data-stage-scroll className="flex-1 overflow-y-auto p-3 flex flex-col gap-3">
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
              aria-busy={pendingIds?.has(job.id) || undefined}
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
                <IconButton data-drag-handle label={`Drag ${job.title} to another stage`}
                  aria-describedby="kanban-drag-instructions" disabled={pendingIds?.has(job.id)}
                  className="cursor-grab touch-none select-none shrink-0"
                  {...handleProps(job)}><Icon name="drag" /></IconButton>
              </div>
              <p className="text-sm">{job.company}</p>
              <p className="text-sm text-[var(--color-ink-mute)]">
                {formatJobLocation(job.location, job.workplace_type, job.locations)}
              </p>
              <p className="text-sm">{formatSalary(job)}</p>
              <label className="flex items-center gap-2 text-xs">Move to
                <select aria-label={`Move ${job.title} to`} value={job.status} disabled={pendingIds?.has(job.id)}
                  className="border p-1 bg-[var(--color-paper)] min-w-0 flex-1"
                  onChange={event => { if (event.target.value !== job.status) onDropJob(job.id, event.target.value as JobStatus); }}>
                  {TRACKED_STAGES.map(stage => <option key={stage} value={stage}>{stage}</option>)}
                </select>
              </label>
              {pendingIds?.has(job.id) && <p role="status" className="text-xs">Saving stage…</p>}
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
