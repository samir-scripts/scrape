import { RemoveJobButton } from "../../ui/RemoveJobButton";
import type { MouseEvent } from "react";
import type { Job } from "../../../bindings/Job";
import { CompanyLogo } from "../../ui/CompanyLogo";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { Tag } from "../../ui/Tag";
import { formatSalary, formatExperience } from "../../../lib/filter";
import { formatJobLocation } from "../../../lib/jobLocationLabel";

interface JobCardProps {
  job: Job;
  selected?: boolean;
  onClick?: () => void;
  onOpenUrl?: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
  onDelete?: (job: Job) => void;
  className?: string;
}

export function JobCard({
  job,
  selected = false,
  onClick,
  onToggleFavorite,
  onDelete,
  className = "",
}: JobCardProps) {
  const salText = formatSalary(job);
  const expText = formatExperience(job);

  const formatDate = (isoString?: string | null) => {
    if (!isoString) return "";
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

  const formattedDate = formatDate(job.date_posted);
  const isRemote =
    job.workplace_type === "remote" ||
    (job.workplace_type === "unknown" &&
      Boolean(job.location && job.location.toLowerCase().includes("remote")));
  const locationLabel = formatJobLocation(
    job.location,
    job.workplace_type,
    job.locations,
  );

  return (
    <div
      className={`group relative flex flex-col gap-2 p-4 border-2 transition-colors select-text ${
        selected
          ? "bg-[var(--color-card)] border-l-4 border-l-[var(--color-ink)] pl-3 border-[var(--color-ink)]"
          : "bg-[var(--color-card)] border-[var(--color-ink)] hover:border-[var(--color-ink)]"
      } ${className}`}
    >
      {/* Top Line: Title + Actions */}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <h2 className="text-base font-bold text-[var(--color-ink)] group-hover:underline leading-snug line-clamp-2">
          <button type="button" onClick={onClick} className="normal-case text-left cursor-pointer" aria-label={`View ${job.title} at ${job.company}`}>
            {job.status === "scraped" && (
              <span className="inline-flex w-3 mr-1 items-center justify-center align-middle text-[var(--color-unread)]" title="New discovery" aria-label="New discovery">
                <svg aria-hidden="true" focusable="false" width="6" height="6" viewBox="0 0 6 6"><circle cx="3" cy="3" r="3" fill="currentColor" /></svg>
              </span>
            )}
            {job.title}
          </button>
        </h2>

        <div className="flex items-center gap-1 shrink-0 pt-0.5">
          {job.status !== "scraped" && job.status !== "viewed" && (
            <Tag status={job.status} />
          )}
          {onToggleFavorite && (
            <FavoriteButton
              favorited={job.favorited}
              onToggle={(e: MouseEvent<HTMLButtonElement>) => {
                e.stopPropagation();
                onToggleFavorite(job.id);
              }}
              size="sm"
            />
          )}
          {onDelete && (
            <RemoveJobButton
              title={job.title}
              company={job.company}
              variant="ghost-danger"
              onClick={e => { e.stopPropagation(); onDelete(job); }}
            />
          )}
        </div>
      </div>

      {/* Second Line: Company + Location on Left, Date on Right */}
      <div className="flex flex-wrap gap-2 items-center justify-between text-xs font-mono text-[var(--color-ink-mute)]">
        <div className="flex items-center gap-2 min-w-0">
          <CompanyLogo
            company={job.company}
            iconUrl={job.company_icon_url} logo={job.company_logo}
            size="sm"
          />
          <span className="font-semibold text-[var(--color-ink)] truncate max-w-[200px]">
            {job.company}
          </span>
          {locationLabel && (
            <>
              <span aria-hidden="true" className="opacity-40">·</span>
              <span className="truncate max-w-[160px]">
                {locationLabel}
              </span>
            </>
          )}
        </div>

        {formattedDate && (
          <span className="shrink-0 text-xs">
            {job.date_posted ? "Posted " : "Found "}{formattedDate}
          </span>
        )}
      </div>

      {/* Badges Row */}
      <div className="flex items-center flex-wrap gap-1.5 mt-0.5 text-xs font-mono">
        {/* Remote Pill Tag (styled with green accent matching reference screenshot) */}
        {isRemote ? (
          <span className="inline-flex items-center px-2 py-0.5 font-bold uppercase rounded-none text-[var(--color-success)] border border-current">
            Remote
          </span>
        ) : job.workplace_type === "onsite" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current font-bold text-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
            IN-OFFICE
          </span>
        ) : job.workplace_type === "hybrid" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current font-bold">
            HYBRID
          </span>
        ) : job.workplace_type !== "unknown" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current font-bold">
            {job.workplace_type}
          </span>
        ) : null}

        {/* Salary */}
        {salText !== "—" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-current text-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
            {salText}
          </span>
        )}

        {/* New Grad */}
        {job.is_new_grad === "yes" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold bg-[var(--color-ink)] text-[var(--color-paper)]">
            NEW GRAD
          </span>
        )}

        {/* Experience */}
        {expText !== "—" && (
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current">
            {expText}
          </span>
        )}

        {/* Role Type */}
        <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current">
          {job.role_type.replace("_", " ")}
        </span>

        {/* Sponsorship */}
        {job.sponsorship === "yes" && (
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current text-[var(--color-success)]">
            SPONSORS
          </span>
        )}
        {job.sponsorship === "no" && (
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current">
            NO VISA
          </span>
        )}

        {/* Work Authorization / Green Card */}
        {job.work_auth === "us_person" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-current text-[var(--color-information)]">
            GC / US PERSON
          </span>
        )}
        {job.work_auth === "citizen_only" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-current text-[var(--color-warning)]">
            CITIZEN ONLY
          </span>
        )}

        {/* Clearance */}
        {job.clearance === "yes" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-[var(--color-danger)] text-[var(--color-danger)]">
            CLR REQ
          </span>
        )}
      </div>

      {/* Description Snippet Preview (2 lines) */}
      {job.description_text && (
        <p className="text-[var(--color-ink-mute)] line-clamp-2 leading-relaxed mt-0.5 font-sans text-sm select-text">
          {job.description_text.replace(/\s+/g, " ").trim()}
        </p>
      )}
    </div>
  );
}
