import type { MouseEvent } from "react";
import type { Job } from "../../../bindings/Job";
import { CompanyLogo } from "../../ui/CompanyLogo";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { Tag } from "../../ui/Tag";
import { formatSalary, formatExperience } from "../../../lib/filter";

interface JobCardProps {
  job: Job;
  selected?: boolean;
  onClick?: () => void;
  onToggleFavorite?: (jobId: string) => void;
  onOpenUrl?: (url: string) => void;
  onDelete?: (job: Job) => void;
  className?: string;
}

export function JobCard({
  job,
  selected = false,
  onClick,
  onToggleFavorite,
  onOpenUrl,
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

  const formattedDate = formatDate(job.date_posted || job.date_added);
  const isRemote =
    job.workplace_type === "remote" ||
    (job.workplace_type === "unknown" &&
      Boolean(job.location && job.location.toLowerCase().includes("remote")));

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick?.();
        }
      }}
      className={`group relative flex flex-col gap-2 p-4 border border-[var(--color-ink)]/20 hover:border-[var(--color-ink)]/50 transition-colors cursor-pointer select-none focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] ${
        selected
          ? "bg-[var(--color-paper-2)]/80 border-l-4 border-l-[var(--color-lime)] pl-3 border-[var(--color-ink)]/60"
          : "bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)]/40"
      } ${className}`}
    >
      {/* Top Line: Title + Status + Favorite */}
      <div className="flex items-start justify-between gap-3">
        <h2 className="text-sm font-bold text-[var(--color-ink)] group-hover:underline leading-snug line-clamp-2">
          {job.title}
        </h2>

        <div className="flex items-center gap-2 shrink-0 pt-0.5">
          <Tag status={job.status} />
          {onOpenUrl && (
            <button
              type="button"
              onClick={(e: MouseEvent<HTMLButtonElement>) => {
                e.stopPropagation();
                onOpenUrl(job.url);
              }}
              title="Open job posting in browser"
              className="px-1.5 py-0.5 border border-[var(--color-ink)]/30 hover:border-[var(--color-ink)] text-[10px] font-mono font-bold text-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] transition-colors cursor-pointer shrink-0"
            >
              [↗]
            </button>
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
            <button
              type="button"
              onClick={(e: MouseEvent<HTMLButtonElement>) => {
                e.stopPropagation();
                onDelete(job);
              }}
              title="Delete job posting"
              className="px-1.5 py-0.5 border border-[var(--color-ink)]/30 hover:border-[var(--color-cherry)] hover:text-[var(--color-cherry)] text-[10px] font-mono font-bold text-[var(--color-ink-mute)] hover:bg-[var(--color-cherry)]/10 transition-colors cursor-pointer shrink-0"
            >
              [DEL]
            </button>
          )}
        </div>
      </div>

      {/* Second Line: Company + Location on Left, Date on Right */}
      <div className="flex items-center justify-between text-xs font-mono text-[var(--color-ink-mute)]">
        <div className="flex items-center gap-2 min-w-0">
          <CompanyLogo
            company={job.company}
            iconUrl={job.company_icon_url}
            size="sm"
          />
          <span className="font-semibold text-[var(--color-ink)] truncate max-w-[200px]">
            {job.company}
          </span>
          {job.location && (
            <>
              <span className="opacity-40">·</span>
              <span className="truncate max-w-[160px] opacity-80">
                {job.location}
              </span>
            </>
          )}
        </div>

        {formattedDate && (
          <span className="shrink-0 text-[11px] opacity-75">
            {formattedDate}
          </span>
        )}
      </div>

      {/* Badges Row */}
      <div className="flex items-center flex-wrap gap-1.5 mt-0.5 text-[10px] font-mono">
        {/* Remote Pill Tag (styled with green accent matching reference screenshot) */}
        {isRemote ? (
          <span className="inline-flex items-center px-2 py-0.5 font-bold uppercase rounded-none bg-emerald-950/40 text-emerald-400 border border-emerald-800/40">
            Remote
          </span>
        ) : job.workplace_type === "onsite" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current font-bold text-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
            IN-OFFICE
          </span>
        ) : job.workplace_type === "hybrid" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current opacity-80 font-bold">
            HYBRID
          </span>
        ) : job.workplace_type !== "unknown" ? (
          <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current opacity-80 font-bold">
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
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current opacity-70">
            {expText}
          </span>
        )}

        {/* Role Type */}
        <span className="inline-flex items-center px-1.5 py-0.5 uppercase border border-current opacity-60">
          {job.role_type.replace("_", " ")}
        </span>

        {/* Sponsorship */}
        {job.sponsorship === "yes" && (
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current opacity-75 text-emerald-400">
            SPONSORS
          </span>
        )}
        {job.sponsorship === "no" && (
          <span className="inline-flex items-center px-1.5 py-0.5 border border-current opacity-50">
            NO VISA
          </span>
        )}

        {/* Work Authorization / Green Card */}
        {job.work_auth === "us_person" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-cyan-800/50 text-cyan-400 bg-cyan-950/30">
            GC / US PERSON
          </span>
        )}
        {job.work_auth === "citizen_only" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-amber-800/50 text-amber-400 bg-amber-950/30">
            CITIZEN ONLY
          </span>
        )}

        {/* Clearance */}
        {job.clearance === "yes" && (
          <span className="inline-flex items-center px-1.5 py-0.5 font-bold border border-[var(--color-cherry)] text-[var(--color-cherry)]">
            CLR REQ
          </span>
        )}
      </div>

      {/* Description Snippet Preview (2 lines) */}
      {job.description_text && (
        <p className="text-[11px] text-[var(--color-ink-mute)] line-clamp-2 leading-relaxed mt-0.5 font-sans opacity-85 select-none">
          {job.description_text.replace(/\s+/g, " ").trim()}
        </p>
      )}
    </div>
  );
}
