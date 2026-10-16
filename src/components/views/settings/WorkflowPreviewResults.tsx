import { ScanDiagnostics } from "./ScanDiagnostics";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../../../bindings/WorkflowPreview";
import { openPostingUrl } from "../../../lib/postingUrl";
export function WorkflowPreviewResults({ preview, draft, setError }: { preview: WorkflowPreview | null; draft: WorkflowSpec; setError: (message: string) => void }) {
 return <>{preview ? (
              <section
                aria-label="Preview results"
                className="grid gap-3 border-t pt-4"
              >
                <h3 className="text-base font-bold">
                  {preview.matched - preview.blocked} eligible matches
                </h3>
                <details>
                  <summary className="cursor-pointer min-h-8">
                    Preview breakdown
                  </summary>
                  <p>
                    {preview.fetched} examined · {preview.matched} matched ·{" "}
                    {preview.rejected} rejected · {preview.blocked} blocked ·{" "}
                    {preview.invalid} invalid
                  </p>
                  <ScanDiagnostics sources={preview.sources} />
                </details>
                {preview.warnings?.map((message, index) => <p role="status" key={`warning-${index}`} className="text-[var(--color-warning)]">Warning: {message}</p>)}
                {preview.errors.map((e, i) => (
                  <p
                    role="alert"
                    key={i}
                    className="text-[var(--color-danger)]"
                  >
                    {e}
                  </p>
                ))}
                {!preview.jobs.length && (
                  <p>
                    No eligible postings. The source may be empty, rules may
                    exclude all postings, or postings may be blocked. You may
                    save a successful preview with zero matches.
                  </p>
                )}
                {preview.sampled && (
                  <p>
                    Preview stopped at its limit or is showing a sample of {preview.jobs.length} eligible postings.
                    Counts cover examined postings only; the source may contain more jobs.
                  </p>
                )}
                {preview.jobs.map(
                  ({ identity, job }) =>
                    job.type === "job" && (
                      <article key={identity} className="border-b py-3">
                        <a
                          href={job.url}
                          onClick={(e) => {
                            e.preventDefault();
                            openPostingUrl(job.url).catch((e) =>
                              setError(String(e)),
                            );
                          }}
                          className="underline font-bold normal-case"
                        >
                          {job.title}
                        </a>
                        <p>
                          {job.company} · {job.location || "Location unknown"}
                        </p>
                      </article>
                    ),
                )}
                <p>
                  Next run:{" "}
                  {preview.next_run_at
                    ? new Date(preview.next_run_at).toLocaleString()
                    : "Manual or inherited schedule"}{" "}
                  · {draft.schedule?.timezone || "Global timezone"}
                </p>
              </section>
            ) : (
              <p className="text-[var(--color-ink-mute)]">
                Preview the current configuration before saving. Preview does
                not save jobs.
              </p>
            )}</>;
}
