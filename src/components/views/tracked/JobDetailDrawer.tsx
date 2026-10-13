import { RemoveJobButton } from "../../ui/RemoveJobButton";
import { useState, useEffect } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import type { JobStatusEvent } from "../../../bindings/JobStatusEvent";
import { api } from "../../../lib/api";
import { Tag } from "../../ui/Tag";
import { Rule } from "../../ui/Rule";
import { KeyHint } from "../../ui/KeyHint";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { CompanyLogo } from "../../ui/CompanyLogo";
import { TRACKED_STAGES } from "../../../lib/constants";
import { useDebounce } from "../../../hooks/useDebounce";

interface JobDetailDrawerProps {
  job: Job | null;
  isOpen: boolean;
  onClose: () => void;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
  onDelete?: (job: Job) => void;
}

export function JobDetailDrawer({
  job,
  isOpen,
  onClose,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onToggleFavorite,
  onDelete,
}: JobDetailDrawerProps) {
  const [events, setEvents] = useState<JobStatusEvent[]>([]);
  const [notes, setNotes] = useState(job?.notes || "");
  const debouncedNotes = useDebounce(notes, 500);

  useEffect(() => {
    setNotes(job?.notes || "");
    if (job?.id) {
      api
        .getJobStatusEvents(job.id)
        .then(setEvents)
        .catch(() => setEvents([]));
    } else {
      setEvents([]);
    }
  }, [job?.id]);

  useEffect(() => {
    if (job && debouncedNotes !== (job.notes || "")) {
      onUpdateNotes(job.id, debouncedNotes);
    }
  }, [debouncedNotes, job?.id]);

  // Capture-phase keydown listener for Escape
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen, onClose]);

  if (!isOpen || !job) return null;

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return iso;
    }
  };

  return (
    <div className="fixed inset-y-0 right-0 w-96 z-40 bg-[var(--color-paper)] border-l-2 border-[var(--color-ink)] flex flex-col shadow-none select-none">
      {/* Drawer Header */}
      <div className="bg-[var(--color-ink)] text-[var(--color-paper)] p-3 flex items-center justify-between">
        <span className="font-mono font-bold text-xs uppercase tracking-wider">
          APPLICATION DETAILS
        </span>
        <button
          onClick={onClose}
          className="text-[var(--color-paper)] hover:opacity-75 cursor-pointer"
        >
          <KeyHint keys="ESC" className="border-[var(--color-paper)] text-xs" />
        </button>
      </div>

      <Rule />

      {/* Drawer Body */}
      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-5 select-text">
        <div className="flex flex-col gap-1">
          <div className="flex items-center justify-between gap-2 mb-1">
            <div className="flex items-center gap-2">
              <Tag status={job.status} />
              <Tag label={job.role_type.toUpperCase()} variant="outline" />
            </div>
            {onToggleFavorite && (
              <div className="flex items-center gap-1.5 font-mono text-xs">
                <FavoriteButton
                  favorited={job.favorited}
                  onToggle={() => onToggleFavorite(job.id)}
                  size="sm"
                />
                <span
                  className={`text-xs font-bold ${
                    job.favorited
                      ? "text-[var(--color-lime)]"
                      : "text-[var(--color-ink-mute)]"
                  }`}
                >
                  {job.favorited ? "FAVORITED" : "FAVORITE"}
                </span>
              </div>
            )}
          </div>
          <div className="flex items-start gap-2.5 mt-1">
            <CompanyLogo
              company={job.company}
              iconUrl={job.company_icon_url}
              size="md"
              className="mt-0.5"
            />
            <div className="flex flex-col min-w-0 flex-1">
              <h2 className="text-xl font-black tracking-tight text-[var(--color-ink)] leading-tight">
                {job.title}
              </h2>
              <div className="text-xs font-mono font-bold text-[var(--color-ink-mute)] mt-0.5">
                {job.company} · {job.location || (job.workplace_type === "onsite" ? "IN-OFFICE" : "REMOTE")} ·{" "}
                {job.workplace_type === "onsite"
                  ? "IN-OFFICE"
                  : job.workplace_type.toUpperCase()}
              </div>
            </div>
          </div>
        </div>

        <Rule />

        {/* Change Stage Buttons */}
        <div className="flex flex-col gap-2 font-mono text-xs">
          <span className="font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
            CHANGE PIPELINE STAGE
          </span>
          <div className="grid grid-cols-2 gap-1.5">
            {TRACKED_STAGES.map((stage) => {
              const isCurrent = job.status === stage;
              return (
                <button
                  key={stage}
                  onClick={() => onUpdateStatus(job.id, stage)}
                  className={`px-2 py-1.5 uppercase font-bold text-xs border cursor-pointer ${
                    isCurrent
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                      : "bg-transparent text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  {stage}
                </button>
              );
            })}
          </div>
        </div>

        <Rule />

        {/* Open Job Link */}
        <div className="flex items-center justify-between p-2 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 font-mono text-xs">
          <a
            href={job.url}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => {
              e.preventDefault();
              onOpenUrl(job.url);
            }}
            title="Open job posting in browser"
            className="truncate max-w-[240px] text-[var(--color-ink)] hover:underline hover:opacity-80 font-mono text-xs flex items-center gap-1.5 cursor-pointer"
          >
            <span className="truncate">{job.url}</span>
            <span className="opacity-60 text-xs shrink-0 font-bold">[↗]</span>
          </a>
          <button
            onClick={() => onOpenUrl(job.url)}
            className="px-2 py-0.5 border border-[var(--color-ink)] font-bold text-xs uppercase hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer shrink-0 ml-2"
          >
            OPEN [↵]
          </button>
        </div>

        {/* Job Description */}
        <div className="flex flex-col gap-1.5 font-mono text-xs">
          <div className="flex justify-between items-center font-bold uppercase tracking-wider">
            <span>JOB DESCRIPTION</span>
            {job.description_text && (
              <span className="text-xs text-[var(--color-ink-mute)] font-normal">
                {job.description_text.length} CHARS
              </span>
            )}
          </div>
          <div className="p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 font-sans text-sm leading-relaxed text-[var(--color-ink)] whitespace-pre-wrap max-h-56 overflow-y-auto select-text">
            {job.description_text ? (
              job.description_text
            ) : (
              <span className="font-mono text-[var(--color-ink-mute)] italic">
                [NO DESCRIPTION EXTRACTED]
              </span>
            )}
          </div>
        </div>

        {/* Notes */}
        <div className="flex flex-col gap-1.5 font-mono text-xs">
          <div className="flex justify-between items-center font-bold uppercase tracking-wider">
            <span>NOTES</span>
            <span className="text-xs text-[var(--color-ink-mute)] font-normal">
              AUTOSAVED
            </span>
          </div>
          <textarea
            rows={5}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="Add notes, follow-up dates, interviewer names..."
            className="w-full p-2.5 font-mono text-xs border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y"
          />
        </div>

        <Rule />

        {/* Status Transition History Timeline */}
        <div className="flex flex-col gap-2 font-mono text-xs">
          <span className="font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
            STAGE HISTORY
          </span>
          {events.length === 0 ? (
            <span className="text-xs text-[var(--color-ink-mute)]">
              NO HISTORY RECORDED
            </span>
          ) : (
            <div className="flex flex-col gap-2 border-l border-[var(--color-ink)] pl-3 ml-1 my-1">
              {events.map((ev) => (
                <div key={ev.id} className="flex flex-col text-xs">
                  <div className="flex items-center gap-1.5 font-bold uppercase">
                    <span className="text-[var(--color-ink-mute)]">
                      {ev.from_status || "NEW"}
                    </span>
                    <span>→</span>
                    <span className="text-[var(--color-ink)]">{ev.to_status}</span>
                  </div>
                  <span className="text-xs text-[var(--color-ink-mute)]">
                    {formatDate(ev.changed_at)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {onDelete && (
          <>
            <Rule />
            <div className="pt-1">
              <RemoveJobButton title={job.title} company={job.company} label="Remove job" onClick={() => onDelete(job)} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
