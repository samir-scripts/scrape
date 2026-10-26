import { runStatusLabel } from "../../../lib/status";
import type { SourceScanStats } from "../../../bindings/SourceScanStats";
import type { RejectionReason } from "../../../bindings/RejectionReason";

const reasons: Record<RejectionReason, string> = {
  query: "Expert query", policy: "Structured policy", legacy_keywords: "Imported include keywords",
  legacy_exclusion: "Imported exclude keywords", legacy_location: "Additional location text",
  legacy_role_type: "Imported role restriction", keyword_any: "No Any keyword matched",
  keyword_all: "Missing required All keywords", keyword_exclusion: "Excluded keyword matched",
  posting_date: "Posting date", compensation: "Compensation", role_type: "Target role or employment type",
  workplace: "Workplace", experience: "Experience", new_grad: "New graduate rule",
  sponsorship: "Sponsorship", work_authorization: "Work authorization", clearance: "Clearance",
  geography: "Country or region", invalid_candidate: "Invalid candidate",
  company: "Company restriction",
};
const terminations: Record<SourceScanStats["termination"], string> = {
  exhausted: "Source exhausted", job_limit: "Run target reached",
  page_limit: "Page limit reached", needs_login: "Needs login",
  request_limit: "Request limit reached", time_limit: "Time limit reached",
  cancelled: "Cancelled", failed: "Source processing failed",
};
export function ScanDiagnostics({ sources }: { sources?: SourceScanStats[] | null }) {
  if (!sources?.length) return null;
  return <div className="grid gap-3">
    {sources.map(source => <section key={source.source_id} aria-label={`Scan details ${source.source_id}`} className="grid gap-2">
      <p>{source.source_id}{source.outcome ? ` · ${runStatusLabel(source.outcome)}` : ""} · {source.termination === "job_limit" && !source.commit_feedback ? "Candidate limit reached" : terminations[source.termination]} · {source.commit_feedback ? `new discoveries ${source.newly_committed ?? 0} (workflow target ${source.limit})` : `candidate limit ${source.limit}`}</p>
      {source.provider_total != null && <p>Provider-reported total: {source.provider_total}</p>}
      <p>{source.examined} examined · {source.accepted} accepted · {source.rejected} matching rejections · {source.blocked} blocked · {source.duplicates} duplicates · {source.invalid} invalid · {source.unresolved ?? 0} unresolved · {source.enrichment_warnings ?? 0} enrichment warnings</p>
      <p className="text-[var(--color-ink-mute)]">Counts describe examined candidates. Accepted candidates passed worker matching, not necessarily persistence or new discovery. Unvisited pages and provider eligibility are not inferred.</p>
      {!!source.rejections.length && <ul className="list-disc pl-5">
        {source.rejections.map(({ reason, count }) => <li key={reason}>{reasons[reason]}: {count}</li>)}
      </ul>}
      {!!source.rejection_examples.length && <details>
        <summary className="cursor-pointer min-h-8">Why representative postings were rejected</summary>
        <p>Primary categories above count each rejection once. Conditions below overlap.</p>
        <ul className="list-disc pl-5">{source.rejection_examples.map((example, i) => <li key={i}>
          {example.title}: {reasons[example.primary]}. Failed conditions: {example.overlapping.map(reason => reasons[reason]).join(", ")}.
        </li>)}</ul>
      </details>}
    </section>)}
  </div>;
}
