import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { Button } from "../../ui/Button";
import { Tag } from "../../ui/Tag";
import { Rule } from "../../ui/Rule";

interface JobMetaPanelProps {
  job: Job | null;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onOpenUrl: (url: string) => void;
}

export function JobMetaPanel({
  job,
  onUpdateStatus,
  onOpenUrl,
}: JobMetaPanelProps) {
  if (!job) {
    return (
      <div className="flex-1 flex items-center justify-center text-center text-xs font-mono text-[var(--color-ink-mute)] uppercase">
        NO METADATA
      </div>
    );
  }

  const formatDate = (isoString?: string | null) => {
    if (!isoString) return "UNKNOWN";
    try {
      const d = new Date(isoString);
      return d.toLocaleDateString(undefined, {
        month: "2-digit",
        day: "2-digit",
        year: "numeric",
      });
    } catch {
      return isoString.slice(0, 10);
    }
  };

  return (
    <div className="flex flex-col gap-5 w-full text-xs font-mono select-none">
      {/* Header */}
      <div className="font-bold uppercase tracking-wider text-[var(--color-ink)] pb-1 border-b border-[var(--color-ink)]">
        STATUS & METADATA
      </div>

      {/* Boxed Metadata Table */}
      <div className="border border-[var(--color-ink)] bg-[var(--color-paper)]">
        <div className="flex justify-between items-center px-3 py-2 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">STATUS</span>
          <Tag status={job.status} />
        </div>
        <div className="flex justify-between items-center px-3 py-2 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">ROLE TYPE</span>
          <span className="font-bold text-[var(--color-ink)]">
            {job.role_type.toUpperCase()}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-2 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">POSTED</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.date_posted)}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-2 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">ADDED</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.date_added)}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-2">
          <span className="text-[var(--color-ink-mute)]">LAST SEEN</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.last_seen_at)}
          </span>
        </div>
      </div>

      <Rule />

      {/* Triage Actions */}
      <div className="flex flex-col gap-2">
        <span className="text-[11px] font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
          TRIAGE ACTIONS
        </span>

        <Button
          variant="inverted"
          onClick={() => onUpdateStatus(job.id, "applied")}
          className="w-full justify-between"
          shortcut="A"
        >
          MARK AS APPLIED
        </Button>

        <Button
          variant="default"
          onClick={() => onUpdateStatus(job.id, "ignored")}
          className="w-full justify-between"
          shortcut="X"
        >
          IGNORE POSTING
        </Button>

        {job.status === "scraped" && (
          <Button
            variant="ghost"
            onClick={() => onUpdateStatus(job.id, "viewed")}
            className="w-full justify-between"
            shortcut="V"
          >
            MARK VIEWED
          </Button>
        )}

        <Button
          variant="ghost"
          onClick={() => onOpenUrl(job.url)}
          className="w-full justify-between mt-2"
          shortcut="↵"
        >
          OPEN IN BROWSER
        </Button>
      </div>
    </div>
  );
}
