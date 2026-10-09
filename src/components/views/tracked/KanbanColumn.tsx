import { useState, type DragEvent } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { Tag } from "../../ui/Tag";

interface KanbanColumnProps {
  status: JobStatus;
  title: string;
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onDropJob: (jobId: string, targetStatus: JobStatus) => void;
}

export function KanbanColumn({
  status,
  title,
  jobs,
  selectedJobId,
  onSelectJob,
  onDropJob,
}: KanbanColumnProps) {
  const [isOver, setIsOver] = useState(false);

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    setIsOver(true);
  };

  const handleDragLeave = () => {
    setIsOver(false);
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    setIsOver(false);
    const jobId = e.dataTransfer.getData("text/plain");
    if (jobId) {
      onDropJob(jobId, status);
    }
  };

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`flex-1 flex flex-col min-w-[240px] max-w-sm h-full border-r border-[var(--color-ink)] last:border-r-0 bg-[var(--color-paper)] transition-colors ${
        isOver ? "bg-[var(--color-paper-2)]" : ""
      }`}
    >
      {/* Column Header */}
      <div className="bg-[var(--color-ink)] text-[var(--color-paper)] p-2.5 flex items-center justify-between select-none">
        <h3 className="font-mono font-bold text-xs uppercase tracking-wider">
          {title}
        </h3>
        <span className="font-mono text-xs px-1.5 py-0.5 bg-[var(--color-paper)] text-[var(--color-ink)] font-bold">
          {jobs.length}
        </span>
      </div>

      {/* Cards List */}
      <div className="flex-1 overflow-y-auto p-2.5 flex flex-col gap-2">
        {jobs.length === 0 ? (
          <div className="h-28 border border-dashed border-[var(--color-ink-mute)] flex items-center justify-center text-center p-3 select-none">
            <span className="text-[11px] font-mono uppercase text-[var(--color-ink-mute)]">
              DROP HERE
            </span>
          </div>
        ) : (
          jobs.map((job) => {
            const isSelected = job.id === selectedJobId;

            return (
              <div
                key={job.id}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("text/plain", job.id);
                }}
                onClick={() => onSelectJob(job)}
                className={`p-3 border border-[var(--color-ink)] cursor-pointer select-none transition-none flex flex-col gap-1.5 ${
                  isSelected
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <div className="flex items-center justify-between gap-1">
                  <span className="font-bold text-xs truncate max-w-[180px]">
                    {job.company}
                  </span>
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
                  <span>{job.location || "REMOTE"}</span>
                  {job.notes && (
                    <span className="font-bold truncate max-w-[80px]">
                      NOTE ▏
                    </span>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
