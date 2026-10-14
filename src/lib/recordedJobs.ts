import capture from "./recordedExample.json";
import type { Job } from "../bindings/Job";
import type { WorkerEvent } from "../bindings/WorkerEvent";
export const recordedProvenance = capture.provenance;
// Presentation of a worker-generated artifact, not a browser scraper.
export function recordedJobs(): Job[] {
  return (capture.events as unknown as WorkerEvent[]).flatMap((event) =>
    event.type !== "job"
      ? []
      : [
          {
            id: `recorded-${event.external_id}`,
            dedupe_key: `${event.source_id}:${event.external_id}`,
            posting_identity: null,
            removed_at: null,
            source_id: event.source_id,
            external_id: event.external_id,
            title: event.title,
            company: event.company,
            role_type: event.role_type,
            location: event.location || null,
            url: event.url,
            date_posted: event.date_posted,
            provider_updated_at: event.provider_updated_at,
            date_added: capture.provenance.captured_at,
            last_seen_at: capture.provenance.captured_at,
            status: "scraped",
            status_changed_at: capture.provenance.captured_at,
            notes: null,
            favorited: false,
            favorited_at: null,
            compensation: event.salary?.ranges || [],
            salary_annual_min: event.salary?.min ?? null,
            salary_annual_max: event.salary?.max ?? null,
            salary_min: event.salary?.min ?? null,
            salary_max: event.salary?.max ?? null,
            salary_currency: event.salary?.currency ?? null,
            salary_interval: event.salary?.ranges[0]?.interval ?? null,
            salary_raw: event.salary?.raw ?? null,
            salary_source: event.salary?.source ?? null,
            workplace_type: event.workplace_type,
            sponsorship: event.sponsorship?.value ?? "unknown",
            sponsorship_evidence: event.sponsorship?.evidence ?? null,
            work_auth: event.work_auth?.value ?? "unknown",
            work_auth_evidence: event.work_auth?.evidence ?? null,
            clearance: event.clearance?.value ?? "unknown",
            clearance_level: event.clearance?.level ?? null,
            clearance_evidence: event.clearance?.evidence ?? null,
            experience_min: event.experience?.min_years ?? null,
            experience_max: event.experience?.max_years ?? null,
            experience_raw: event.experience?.raw ?? null,
            is_new_grad: event.is_new_grad,
            experience_evidence: event.experience?.evidence ?? null,
            description_text: event.description_text,
            company_icon_url: event.company_icon_url,
            enrichment_version: 1,
            locations: event.locations,
          },
        ],
  );
}
