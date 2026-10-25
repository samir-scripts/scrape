import { Icon } from "../../ui/Icon";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { Button } from "../../ui/Button";
import { Tag } from "../../ui/Tag";
import { Rule } from "../../ui/Rule";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { CompanyLogo } from "../../ui/CompanyLogo";
import {
  formatSalary,
  formatExperience,
  triStateToLabel,
} from "../../../lib/filter";

interface JobMetaPanelProps {
  job: Job | null;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
}

export function JobMetaPanel({
  job,
  onUpdateStatus,
  onOpenUrl,
  onToggleFavorite,
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

  const salDisplay = formatSalary(job);
  const expDisplay = formatExperience(job);

  return (
    <div className="flex flex-col gap-4 w-full text-xs font-mono select-none overflow-y-auto">
      {/* Header */}
      <div className="font-bold uppercase tracking-wider text-[var(--color-ink)] pb-1 border-b border-[var(--color-ink)]">
        STATUS & METADATA
      </div>

      {/* Boxed Metadata Table */}
      <div className="border border-[var(--color-ink)] bg-[var(--color-paper)]">
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">COMPANY</span>
          <div className="flex items-center gap-1.5 font-bold text-[var(--color-ink)]">
            <CompanyLogo company={job.company} jobId={job.id} sourceId={job.source_id} logo={job.company_logo} size="lg" />
            <span className="truncate max-w-[140px]">{job.company}</span>
          </div>
        </div>
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">STATUS</span>
          <Tag status={job.status} />
        </div>
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">FAVORITED</span>
          <div className="flex items-center gap-1.5">
            <FavoriteButton
              favorited={job.favorited}
              onToggle={() => onToggleFavorite?.(job.id)}
              size="sm"
            />
            <span
              className={`font-bold ${
                job.favorited ? "text-[var(--color-lime)]" : "text-[var(--color-ink-mute)]"
              }`}
            >
              {job.favorited ? "PROTECTED" : "NO"}
            </span>
          </div>
        </div>
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">ROLE TYPE</span>
          <span className="font-bold text-[var(--color-ink)]">
            {job.role_type.toUpperCase()}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">POSTED</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.date_posted)}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-1.5 border-b border-[var(--color-ink)]">
          <span className="text-[var(--color-ink-mute)]">ADDED</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.date_added)}
          </span>
        </div>
        <div className="flex justify-between items-center px-3 py-1.5">
          <span className="text-[var(--color-ink-mute)]">LAST SEEN</span>
          <span className="text-[var(--color-ink)]">
            {formatDate(job.last_seen_at)}
          </span>
        </div>
      </div>

      <Rule />

      {/* Compensation & Workplace */}
      <div className="flex flex-col gap-2">
        <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
          COMPENSATION & GEOGRAPHY
        </span>
        <div className="border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)]">
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">SALARY</span>
            <span className="font-bold text-[var(--color-ink)]">
              {salDisplay}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">WORKPLACE</span>
            <span className="font-bold uppercase text-[var(--color-ink)]">
              {job.workplace_type === "onsite"
                ? "IN-OFFICE"
                : job.workplace_type}
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
      </div>

      <Rule />

      {/* Eligibility & Experience */}
      <div className="flex flex-col gap-2">
        <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
          REQUIREMENTS & CLEARANCE
        </span>
        <div className="border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)]">
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">EXPERIENCE</span>
            <span className="font-bold text-[var(--color-ink)]">
              {expDisplay}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">NEW GRAD</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.is_new_grad)}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">SPONSORSHIP</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.sponsorship)}
            </span>
          </div>
          <div className="flex justify-between items-center px-3 py-1.5">
            <span className="text-[var(--color-ink-mute)]">CLEARANCE</span>
            <span className="font-bold text-[var(--color-ink)]">
              {triStateToLabel(job.clearance)}
              {job.clearance_level && ` (${job.clearance_level})`}
            </span>
          </div>
        </div>

        {/* Evidence Snippets */}
        {(job.experience_evidence ||
          job.sponsorship_evidence ||
          job.clearance_evidence) && (
          <div className="flex flex-col gap-1.5 mt-1">
            <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
              EVIDENCE QUOTES
            </span>
            {job.experience_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)]/60 bg-[var(--color-paper-2)]/30 text-xs italic leading-tight">
                <span className="font-bold not-italic text-[9px] block text-[var(--color-ink-mute)]">
                  EXP QUOTE:
                </span>
                "{job.experience_evidence}"
              </div>
            )}
            {job.sponsorship_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)]/60 bg-[var(--color-paper-2)]/30 text-xs italic leading-tight">
                <span className="font-bold not-italic text-[9px] block text-[var(--color-ink-mute)]">
                  SPONSORSHIP QUOTE:
                </span>
                "{job.sponsorship_evidence}"
              </div>
            )}
            {job.clearance_evidence && (
              <div className="p-2 border border-dashed border-[var(--color-ink)]/60 bg-[var(--color-paper-2)]/30 text-xs italic leading-tight">
                <span className="font-bold not-italic text-[9px] block text-[var(--color-ink-mute)]">
                  CLEARANCE QUOTE:
                </span>
                "{job.clearance_evidence}"
              </div>
            )}
          </div>
        )}
      </div>

      <Rule />

      {/* Triage Actions */}
      <div className="flex flex-col gap-2 pb-2">
        <span className="text-xs font-bold text-[var(--color-ink-mute)] uppercase tracking-wider">
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

        {onToggleFavorite && (
          <Button
            variant={job.favorited ? "inverted" : "default"}
            onClick={() => onToggleFavorite(job.id)}
            className="w-full justify-between"
            shortcut="S"
          >
            <span className="inline-flex items-center gap-2"><Icon name="star" filled={job.favorited} />{job.favorited ? "FAVORITED (PROTECTED)" : "FAVORITE POSTING"}</span>
          </Button>
        )}

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
          className="w-full justify-between mt-1"
          shortcut="↵"
        >
          OPEN IN BROWSER
        </Button>
      </div>
    </div>
  );
}
