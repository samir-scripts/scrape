import type { CSSProperties, ReactNode } from "react";
import type { Job } from "../../../bindings/Job";
import { STATUS_CONFIG } from "../../../lib/constants";
import { formatSalary } from "../../../lib/filter";
import { formatJobLocation } from "../../../lib/jobLocationLabel";

export function KanbanCard({ job, title, actions, children, footer, id, selected = false, placeholder = false, preview = false }: {
  job: Job;
  id?: string;
  selected?: boolean;
  title: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  placeholder?: boolean;
  preview?: boolean;
}) {
  const accent = STATUS_CONFIG[job.status]?.stageAccent;
  return <article id={id} aria-hidden={preview || undefined}
    className={`border border-l-4 p-4 grid gap-3 select-text bg-[var(--color-paper)] ${selected ? "outline outline-2 outline-[var(--color-focus)] outline-offset-[-2px]" : ""} ${placeholder ? "opacity-25 border-dashed" : ""} ${preview ? "pointer-events-none outline outline-2 outline-[var(--color-ink)]" : ""}`}
    style={{ borderLeftColor: accent, ...(preview ? { backgroundColor: "var(--color-paper)" } : {}) } as CSSProperties}>
    <div className="flex items-start justify-between gap-2">{title}{!preview && actions}</div>
    <p className="text-sm">{job.company}</p>
    <p className="text-sm text-[var(--color-ink-mute)]">{formatJobLocation(job.location, job.workplace_type, job.locations)}</p>
    <p className="text-sm">{formatSalary(job)}</p>
    {!preview && children}
    {preview ? <div className="flex items-center gap-2 text-xs"><span>Move to</span><span className="border px-2 py-1 uppercase">{job.status}</span></div> : footer}
  </article>;
}

export function KanbanCardPreview({ job }: { job: Job }) {
  return <KanbanCard job={job} preview title={<span className="normal-case text-left font-bold text-base leading-snug line-clamp-2">{job.title}</span>} />;
}
