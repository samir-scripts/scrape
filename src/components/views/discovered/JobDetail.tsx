import { useState, useEffect } from "react";
import type { Job } from "../../../bindings/Job";
import { Tag } from "../../ui/Tag";
import { Rule } from "../../ui/Rule";
import { Dither } from "../../ui/Dither";
import { useDebounce } from "../../../hooks/useDebounce";

interface JobDetailProps {
  job: Job | null;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
}

export function JobDetail({ job, onUpdateNotes, onOpenUrl }: JobDetailProps) {
  const [notes, setNotes] = useState(job?.notes || "");
  const debouncedNotes = useDebounce(notes, 500);

  // Sync state when job changes
  useEffect(() => {
    setNotes(job?.notes || "");
  }, [job?.id]);

  // Autosave notes
  useEffect(() => {
    if (job && debouncedNotes !== (job.notes || "")) {
      onUpdateNotes(job.id, debouncedNotes);
    }
  }, [debouncedNotes, job?.id]);

  if (!job) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-center p-8 text-[var(--color-ink-mute)]">
        <Dither type="checker" width="60px" height="24px" className="mb-3" />
        <p className="font-mono text-xs uppercase tracking-wider">
          NO POSTING SELECTED
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5 w-full select-text">
      {/* Role Type & Status Tags */}
      <div className="flex items-center gap-2">
        <Tag label={job.role_type.toUpperCase()} variant="solid" />
        <Tag status={job.status} />
      </div>

      {/* Main Title & Company */}
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-black uppercase tracking-tight text-[var(--color-ink)] leading-tight">
          {job.title}
        </h1>
        <div className="flex items-center gap-3 text-xs font-mono text-[var(--color-ink-mute)] uppercase tracking-wider">
          <span className="font-bold text-[var(--color-ink)]">{job.company}</span>
          <span>·</span>
          <span>{job.location || "REMOTE / UNSPECIFIED"}</span>
        </div>
      </div>

      <Rule />

      {/* External Link */}
      <div className="flex items-center justify-between p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono text-xs">
        <span className="truncate max-w-md text-[var(--color-ink)] select-all">
          {job.url}
        </span>
        <button
          onClick={() => onOpenUrl(job.url)}
          className="px-2.5 py-1 bg-[var(--color-ink)] text-[var(--color-paper)] font-bold text-[11px] uppercase tracking-wider cursor-pointer hover:opacity-90"
        >
          [↵] OPEN POSTING
        </button>
      </div>

      {/* Notes Area */}
      <div className="flex flex-col gap-2 mt-2">
        <div className="flex justify-between items-center text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
          <span>CANDIDATE NOTES ▏</span>
          <span className="text-[10px] text-[var(--color-ink-mute)] font-normal">
            AUTOSAVES (500MS)
          </span>
        </div>
        <textarea
          rows={6}
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder="ENTER APPLICATION NOTES, SALARY RANGE, CONTACTS, OR THOUGHTS..."
          className="w-full p-3 font-mono text-xs border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y"
        />
      </div>
    </div>
  );
}
