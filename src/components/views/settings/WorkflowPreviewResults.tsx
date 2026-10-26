import { ScanDiagnostics } from "./ScanDiagnostics";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../../../bindings/WorkflowPreview";
import { openPostingUrl } from "../../../lib/postingUrl";
import { CompanyLogo } from "../../ui/CompanyLogo";
export function WorkflowPreviewResults({ preview, draft, setError }: { preview: WorkflowPreview | null; draft: WorkflowSpec; setError: (message: string) => void }) {
  return <>{preview ? (
    <section
      aria-label="Preview results"
      className="grid gap-3 border-t border-[var(--color-border)] pt-4"
    >
      <h3 className="text-base font-bold">
        {preview.matched - preview.blocked} eligible matches
      </h3>
      <details className="border border-[var(--color-border)] p-2 bg-[var(--color-surface)]">
        <summary className="cursor-pointer min-h-8 font-semibold text-xs uppercase">
          Preview breakdown
        </summary>
        <div className="pt-2 grid gap-2">
          <p className="text-xs">
            {preview.fetched} examined · {preview.matched} matched ·{" "}
            {preview.rejected} rejected · {preview.blocked} blocked ·{" "}
            {preview.invalid} invalid
          </p>
          <ScanDiagnostics sources={preview.sources} />
        </div>
      </details>
      {preview.warnings?.map((message, index) => <p role="status" key={`warning-${index}`} className="text-[var(--color-warning)] text-xs">Warning: {message}</p>)}
      {preview.errors.map((e, i) => (
        <p
          role="alert"
          key={i}
          className="text-[var(--color-danger)] text-xs font-semibold"
        >
          {e}
        </p>
      ))}
      {!preview.jobs.length && (
        <p className="text-xs text-[var(--color-text-muted)] p-3 border border-[var(--color-border)] bg-[var(--color-surface)]">
          No eligible postings. The source may be empty, rules may
          exclude all postings, or postings may be blocked. You may
          save a successful preview with zero matches.
        </p>
      )}
      {preview.sampled && (
        <p className="text-xs text-[var(--color-text-muted)]">
          Preview stopped at its page, time, request, or result limit. Counts cover examined postings only; more jobs may exist.
          Preview-only limits do not change saved source limits.
        </p>
      )}
      <div className="grid gap-2">
        {preview.jobs.map(
          ({ identity, job }) =>
            job.type === "job" && (
              <article key={identity} className="border-2 border-[var(--color-border)] p-3 bg-[var(--color-card)] grid gap-2">
                <div className="flex items-start gap-3">
                  <CompanyLogo company={job.company} size="sm" logo={preview.branding.find(item => item.source_id === job.source_id && item.company === job.company)?.logo} />
                  <div className="min-w-0 flex-1">
                    <a
                      href={job.url}
                      onClick={(e) => {
                        e.preventDefault();
                        openPostingUrl(job.url).catch((e) =>
                          setError(String(e)),
                        );
                      }}
                      className="underline font-bold normal-case hover:opacity-80"
                    >
                      {job.title}
                    </a>
                    <p className="text-xs text-[var(--color-text-muted)]">
                      {job.company} · {job.location || "Location unknown"}
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap gap-1 text-[11px] text-[var(--color-text-muted)]">
                  <span className="px-1.5 py-0.5 border border-[var(--color-border)] uppercase">Employment: {job.role_type.replaceAll("_", " ")}</span>
                  <span className="px-1.5 py-0.5 border border-[var(--color-border)]">Required experience: {job.experience?.min_years == null ? "Unknown" : `${job.experience.min_years}+ years`}</span>
                  <span className="px-1.5 py-0.5 border border-[var(--color-border)]">New Grad: {job.is_new_grad}</span>
                </div>
                {job.experience?.evidence && <p className="text-[11px] text-[var(--color-text-muted)]">Experience evidence: {job.experience.evidence}</p>}
                <p className="text-[10px] text-[var(--color-text-muted)]">Logo: {preview.branding.find(item => item.source_id === job.source_id && item.company === job.company)?.logo.diagnostic || preview.branding.find(item => item.source_id === job.source_id && item.company === job.company)?.logo.status.replaceAll("_", " ") || "No employer logo metadata"}</p>
              </article>
            ),
        )}
      </div>
      <p className="text-xs text-[var(--color-text-muted)]">
        Next run:{" "}
        {preview.next_run_at
          ? new Date(preview.next_run_at).toLocaleString()
          : "Manual or inherited schedule"}{" "}
        · {draft.schedule?.timezone || "Global timezone"}
      </p>
    </section>
  ) : (
    <p className="text-[var(--color-text-muted)]">
      Preview the current configuration before saving. Preview does
      not save jobs.
    </p>
  )}</>;
}
