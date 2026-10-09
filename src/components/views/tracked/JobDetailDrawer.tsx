import { useState, useEffect } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import type { JobStatusEvent } from "../../../bindings/JobStatusEvent";
import { api } from "../../../lib/api";
import { Tag } from "../../ui/Tag";
import { Button } from "../../ui/Button";
import { Rule } from "../../ui/Rule";
import { KeyHint } from "../../ui/KeyHint";
import { TRACKED_STAGES } from "../../../lib/constants";
import { useDebounce } from "../../../hooks/useDebounce";

interface JobDetailDrawerProps {
  job: Job | null;
  isOpen: boolean;
  onClose: () => void;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
}

export function JobDetailDrawer({
  job,
  isOpen,
  onClose,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
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
          <KeyHint keys="ESC" className="border-[var(--color-paper)] text-[10px]" />
        </button>
      </div>

      <Rule />

      {/* Drawer Body */}
      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-5 select-text">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2 mb-1">
            <Tag status={job.status} />
            <Tag label={job.role_type.toUpperCase()} variant="outline" />
          </div>
          <h2 className="text-xl font-black uppercase tracking-tight text-[var(--color-ink)]">
            {job.title}
          </h2>
          <div className="text-xs font-mono font-bold text-[var(--color-ink-mute)]">
            {job.company} · {job.location || "REMOTE"}
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
                  className={`px-2 py-1.5 uppercase font-bold text-[11px] border cursor-pointer ${
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
        <Button
          variant="inverted"
          onClick={() => onOpenUrl(job.url)}
          className="w-full justify-center"
        >
          [↵] OPEN JOB POSTING
        </Button>

        {/* Notes */}
        <div className="flex flex-col gap-1.5 font-mono text-xs">
          <div className="flex justify-between items-center font-bold uppercase tracking-wider">
            <span>NOTES</span>
            <span className="text-[10px] text-[var(--color-ink-mute)] font-normal">
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
            <span className="text-[11px] text-[var(--color-ink-mute)]">
              NO HISTORY RECORDED
            </span>
          ) : (
            <div className="flex flex-col gap-2 border-l border-[var(--color-ink)] pl-3 ml-1 my-1">
              {events.map((ev) => (
                <div key={ev.id} className="flex flex-col text-[11px]">
                  <div className="flex items-center gap-1.5 font-bold uppercase">
                    <span className="text-[var(--color-ink-mute)]">
                      {ev.from_status || "NEW"}
                    </span>
                    <span>→</span>
                    <span className="text-[var(--color-ink)]">{ev.to_status}</span>
                  </div>
                  <span className="text-[10px] text-[var(--color-ink-mute)]">
                    {formatDate(ev.changed_at)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
