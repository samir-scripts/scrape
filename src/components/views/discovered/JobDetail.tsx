import { RemoveJobButton } from "../../ui/RemoveJobButton";
import { useState, useEffect } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { Tag } from "../../ui/Tag";
import { Rule } from "../../ui/Rule";
import { Button } from "../../ui/Button";
import { Dither } from "../../ui/Dither";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { CompanyLogo } from "../../ui/CompanyLogo";
import { useDebounce } from "../../../hooks/useDebounce";
import {
  formatSalary,
  formatExperience,
  formatWorkAuth,
  triStateToLabel,
} from "../../../lib/filter";

interface JobDetailProps {
  job: Job | null;
  onClose?: () => void;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onUpdateNotes: (jobId: string, notes: string) => void;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
  onDelete?: (job: Job) => void;
}

export function JobDetail({
  job,
  onClose,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onToggleFavorite,
  onDelete,
}: JobDetailProps) {
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
  }, [debouncedNotes, job?.id, onUpdateNotes]);

  // Capture Escape key to close window
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose?.();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

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

  const formatDate = (isoString?: string | null) => {
    if (!isoString) return "UNKNOWN";
    try {
      const d = new Date(isoString);
      return d.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
        timeZone: "UTC",
      });
    } catch {
      return isoString.slice(0, 10);
    }
  };

  const salDisplay = formatSalary(job);
  const expDisplay = formatExperience(job);

  return (
    <div className="flex flex-col gap-5 w-full p-6 select-text">
      {/* 1. Window Controls Header */}
      <div className="flex items-center justify-between pb-2 border-b border-[var(--color-ink)] font-mono text-xs">
        <div className="flex items-center gap-2">
          <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
            JOB DETAILS ▏
          </span>
          <Tag status={job.status} />
          <Tag label={job.role_type.replace("_", " ").toUpperCase()} variant="outline" />
        </div>

        <div className="flex items-center gap-2">
          {onToggleFavorite && (
            <FavoriteButton
              favorited={job.favorited}
              onToggle={() => onToggleFavorite(job.id)}
              size="md"
            />
          )}

          <button
            onClick={() => onOpenUrl(job.url)}
            title="Open in external browser [O]"
            className="px-2.5 py-1 bg-[var(--color-ink)] text-[var(--color-paper)] font-bold text-xs uppercase tracking-wider cursor-pointer hover:opacity-90 shrink-0"
          >
            [↵] OPEN
          </button>

          {onClose && (
            <button
              onClick={onClose}
              title="Close job window [ESC]"
              className="px-2 py-1 border border-[var(--color-ink)] text-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] font-bold text-xs uppercase tracking-wider cursor-pointer shrink-0 transition-colors"
            >
              [✕] CLOSE [ESC]
            </button>
          )}
        </div>
      </div>

      {/* 2. Triage Actions Toolbar */}
      <div className="flex items-center flex-wrap gap-2 p-2 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 font-mono text-xs">
        <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase mr-1">
          TRIAGE:
        </span>
        <Button
          variant="inverted"
          onClick={() => onUpdateStatus(job.id, "applied")}
          shortcut="A"
        >
          MARK AS APPLIED
        </Button>

        <Button
          variant="default"
          onClick={() => onUpdateStatus(job.id, "ignored")}
          shortcut="X"
        >
          IGNORE POSTING
        </Button>

        {job.status === "scraped" && (
          <Button
            variant="ghost"
            onClick={() => onUpdateStatus(job.id, "viewed")}
            shortcut="V"
          >
            MARK VIEWED
          </Button>
        )}

        {onToggleFavorite && (
          <Button
            variant={job.favorited ? "inverted" : "ghost"}
            onClick={() => onToggleFavorite(job.id)}
            shortcut="S"
          >
            {job.favorited ? "★ FAVORITED" : "☆ FAVORITE"}
          </Button>
        )}

        {onDelete && <RemoveJobButton title={job.title} company={job.company} label="Remove job" onClick={() => onDelete(job)} />}
      </div>

      {/* 3. Main Title & Company Info */}
      <div className="flex items-start gap-4">
        <CompanyLogo
          company={job.company}
          iconUrl={job.company_icon_url}
          size="lg"
          className="mt-1"
        />
        <div className="flex flex-col gap-1 min-w-0 flex-1">
          <h1 className="text-xl md:text-2xl font-black tracking-tight text-[var(--color-ink)] leading-tight">
            {job.title}
          </h1>
          <div className="flex items-center gap-2 text-xs font-mono text-[var(--color-ink-mute)] uppercase tracking-wider">
            <span className="font-bold text-[var(--color-ink)]">
              {job.company}
            </span>
            <span>·</span>
            <span>{job.location || (job.workplace_type === "onsite" ? "IN-OFFICE" : "REMOTE / UNSPECIFIED")}</span>
            <span>·</span>
            <span className="font-semibold text-[var(--color-ink)]">
              {job.workplace_type === "onsite"
                ? "IN-OFFICE"
                : job.workplace_type.toUpperCase()}
            </span>
          </div>
        </div>
      </div>

      {/* 4. External URL bar */}
      <div className="flex items-center justify-between p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 font-mono text-xs">
        <a
          href={job.url}
          target="_blank"
          rel="noopener noreferrer"
          onClick={(e) => {
            e.preventDefault();
            onOpenUrl(job.url);
          }}
          title="Open job posting in browser"
          className="truncate max-w-md text-[var(--color-ink)] hover:underline hover:opacity-80 font-mono text-xs flex items-center gap-1.5 cursor-pointer"
        >
          <span className="truncate">{job.url}</span>
          <span className="opacity-60 text-xs shrink-0 font-bold">[↗]</span>
        </a>
        <button
          onClick={() => onOpenUrl(job.url)}
          className="px-2 py-0.5 border border-[var(--color-ink)] font-bold text-xs uppercase hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer shrink-0 ml-2"
        >
          OPEN POSTING
        </button>
      </div>

      {/* 5. Consolidated Metadata Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 font-mono text-xs">
        {/* Box A: Compensation & Geography */}
        <div className="flex flex-col border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)]">
          <div className="px-3 py-1.5 bg-[var(--color-paper-2)]/50 font-bold uppercase tracking-wider text-xs text-[var(--color-ink)]">
            COMPENSATION & WORKPLACE
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">SALARY</span>
            <span className="font-bold text-[var(--color-ink)]">{salDisplay}</span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">WORKPLACE</span>
            <span className="font-bold uppercase text-[var(--color-ink)]">
              {job.workplace_type === "onsite"
                ? "IN-OFFICE"
                : job.workplace_type}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">ROLE TYPE</span>
            <span className="font-bold text-[var(--color-ink)]">
              {job.role_type.replace("_", " ").toUpperCase()}
            </span>
          </div>
          {job.locations && job.locations.length > 0 && (
            <div className="px-3 py-1.5 flex flex-col gap-1">
              <span className="text-xs text-[var(--color-ink-mute)] uppercase">
                LOCATIONS ({job.locations.length})
              </span>
              <div className="flex flex-wrap gap-1">
                {job.locations.map((loc, idx) => (
                  <span
                    key={idx}
                    className="px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-xs"
                  >
                    {[loc.city, loc.region_code, loc.country_code].filter(Boolean).join(", ") || loc.raw}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Box B: Requirements & Eligibility */}
        <div className="flex flex-col border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)]">
          <div className="px-3 py-1.5 bg-[var(--color-paper-2)]/50 font-bold uppercase tracking-wider text-xs text-[var(--color-ink)]">
            REQUIREMENTS & CLEARANCE
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">EXPERIENCE</span>
            <span className="font-bold text-[var(--color-ink)]">{expDisplay}</span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">NEW GRAD</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.is_new_grad)}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">VISA SPONSORSHIP</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.sponsorship)}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">WORK AUTH / GREEN CARD</span>
            <span className="font-bold text-[var(--color-ink)]">
              {formatWorkAuth(job)}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">SECURITY CLEARANCE</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.clearance)}
              {job.clearance_level && ` (${job.clearance_level})`}
            </span>
          </div>
        </div>
      </div>

      {/* 6. Posting Telemetry & Dates */}
      <div className="border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)] font-mono text-xs">
        <div className="px-3 py-1.5 bg-[var(--color-paper-2)]/30 font-bold uppercase tracking-wider text-xs text-[var(--color-ink)]">
          POSTING TIMELINE
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 divide-y sm:divide-y-0 sm:divide-x divide-[var(--color-ink)]">
          <div className="flex justify-between sm:flex-col gap-1 px-3 py-1.5">
            <span className="text-xs text-[var(--color-ink-mute)] uppercase">POSTED DATE</span>
            <span className="font-bold text-[var(--color-ink)]">{formatDate(job.date_posted)}</span>
          </div>
          <div className="flex justify-between sm:flex-col gap-1 px-3 py-1.5">
            <span className="text-xs text-[var(--color-ink-mute)] uppercase">DISCOVERED AT</span>
            <span className="font-bold text-[var(--color-ink)]">{formatDate(job.date_added)}</span>
          </div>
          <div className="flex justify-between sm:flex-col gap-1 px-3 py-1.5">
            <span className="text-xs text-[var(--color-ink-mute)] uppercase">LAST SEEN</span>
            <span className="font-bold text-[var(--color-ink)]">{formatDate(job.last_seen_at)}</span>
          </div>
        </div>
      </div>

      {/* 7. Evidence Snippets (if any) */}
      {(job.experience_evidence || job.sponsorship_evidence || job.work_auth_evidence || job.clearance_evidence) && (
        <div className="flex flex-col gap-2 font-mono text-xs">
          <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
            POLICY ENGINE EVIDENCE QUOTES
          </span>
          <div className="flex flex-col gap-1.5">
            {job.experience_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-xs leading-tight">
                <span className="font-bold text-xs block text-[var(--color-ink-mute)] uppercase mb-0.5">
                  EXPERIENCE QUOTE:
                </span>
                "{job.experience_evidence}"
              </div>
            )}
            {job.sponsorship_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-xs leading-tight">
                <span className="font-bold text-xs block text-[var(--color-ink-mute)] uppercase mb-0.5">
                  SPONSORSHIP QUOTE:
                </span>
                "{job.sponsorship_evidence}"
              </div>
            )}
            {job.work_auth_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-xs leading-tight">
                <span className="font-bold text-xs block text-[var(--color-ink-mute)] uppercase mb-0.5">
                  WORK AUTH / GREEN CARD QUOTE:
                </span>
                "{job.work_auth_evidence}"
              </div>
            )}
            {job.clearance_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-xs leading-tight">
                <span className="font-bold text-xs block text-[var(--color-ink-mute)] uppercase mb-0.5">
                  CLEARANCE QUOTE:
                </span>
                "{job.clearance_evidence}"
              </div>
            )}
          </div>
        </div>
      )}

      {/* 8. Full Job Description Card */}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between items-center text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
          <span>JOB DESCRIPTION ▏</span>
          {job.description_text && (
            <span className="text-xs text-[var(--color-ink-mute)] font-normal">
              {job.description_text.length.toLocaleString()} CHARACTERS
            </span>
          )}
        </div>
        <div className="p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 font-sans text-sm leading-relaxed text-[var(--color-ink)] whitespace-pre-wrap max-h-96 overflow-y-auto select-text">
          {job.description_text ? (
            job.description_text
          ) : (
            <span className="font-mono text-[var(--color-ink-mute)] italic">
              [NO PLAIN-TEXT DESCRIPTION EXTRACTED BY SCRAPER]
            </span>
          )}
        </div>
      </div>

      <Rule />

      {/* 9. Candidate Notes Section */}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between items-center text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
          <span>CANDIDATE NOTES ▏</span>
          <span className="text-xs text-[var(--color-ink-mute)] font-normal">
            AUTOSAVED (500MS)
          </span>
        </div>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder="ENTER APPLICATION NOTES, REFERRAL DETAILS, RECRUITER CONTACTS, INTERVIEW STAGES..."
          className="w-full h-24 p-3 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] font-mono text-xs focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y placeholder:text-[var(--color-ink-mute)]"
        />
      </div>
    </div>
  );
}
