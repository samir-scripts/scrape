import { CompanyLogo } from "../ui/CompanyLogo";
import { useEffect, useRef, useState } from "react";
import type { Job } from "../../bindings/Job";
import type { JobStatus } from "../../bindings/JobStatus";
import type { JobStatusEvent } from "../../bindings/JobStatusEvent";
import { api } from "../../lib/api";
import { TRACKED_STAGES } from "../../lib/constants";
import {
  formatSalary,
  formatExperience,
  formatWorkAuth,
  triStateToLabel,
} from "../../lib/filter";
import { Button } from "../ui/Button";
import { FavoriteButton } from "../ui/FavoriteButton";
import { RemoveJobButton } from "../ui/RemoveJobButton";
import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
import { Modal } from "../ui/Modal";
import { JobDescriptionView } from "./JobDescriptionView";
import { Select } from "../ui/Select";
import { ActionMenu } from "../ui/ActionMenu";
import { formatJobLocation } from "../../lib/jobLocationLabel";

export interface JobInspectorProps {
  job: Job | null;
  onClose?: () => void;
  mode?: "inline" | "modal";
  onResetPane?: () => void;
  onUpdateStatus: (id: string, status: JobStatus) => void;
  onUpdateNotes: (id: string, notes: string) => void | Promise<unknown>;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (id: string) => void;
  onDelete?: (job: Job) => void;
}
const date = (value: string | null | undefined) =>
  value ? new Date(value).toLocaleString() : "Unknown";
export function JobInspectorPanel({ mode = "inline", ...props }: JobInspectorProps) {
  if (!props.job) return null;
  return <Modal isOpen title="Job details" inline={mode === "inline"} width="max-w-[640px]" onClose={() => props.onClose?.()}>
    <JobInspectorContent key={props.job.id} {...props} job={props.job} onClose={mode === "inline" ? props.onClose : undefined} />
  </Modal>;
}
export function JobInspectorContent({
  job,
  onClose,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onToggleFavorite,
  onDelete,
  onResetPane,
}: JobInspectorProps & { job: Job }) {
  const [notes, setNotes] = useState(job.notes || "");
  const [noteState, setNoteState] = useState("Saved");
  const [events, setEvents] = useState<JobStatusEvent[]>([]);
  const [historyError, setHistoryError] = useState("");
  const [historyOpen, setHistoryOpen] = useState(false);
  const mounted = useRef(true),
    latest = useRef(notes),
    persisted = useRef(notes);
  const update = useRef(onUpdateNotes);
  update.current = onUpdateNotes;
  const queue = useRef(Promise.resolve());
  const save = (text: string) => {
    setNoteState("Saving");
    queue.current = queue.current
      .catch(() => {})
      .then(async () => {
        await update.current(job.id, text);
        persisted.current = text;
      })
      .then(() => {
        if (mounted.current && latest.current === text) setNoteState("Saved");
      })
      .catch(() => {
        if (mounted.current) setNoteState("Failed to save");
      });
  };
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (latest.current !== persisted.current) {
        const text = latest.current;
        queue.current
          .catch(() => {})
          .then(() => update.current(job.id, text))
          .catch(() => {});
      }
    };
  }, [job.id]);
  useEffect(() => {
    if (notes === persisted.current) return;
    const timer = setTimeout(() => save(notes), 500);
    return () => clearTimeout(timer);
  }, [notes]);
  useEffect(() => {
    if (!historyOpen) return;
    let active = true;
    setHistoryError("");
    api
      .getJobStatusEvents(job.id)
      .then((result) => {
        if (active) setEvents(result);
      })
      .catch((error) => {
        if (active) setHistoryError(String(error));
      });
    return () => {
      active = false;
    };
  }, [job.id, job.status, historyOpen]);
  const tracked = TRACKED_STAGES.includes(job.status);
  const evidence = [
    ["Sponsorship", job.sponsorship_evidence],
    ["Work authorization", job.work_auth_evidence],
    ["Clearance", job.clearance_evidence],
    ["Experience", job.experience_evidence],
    ["Compensation", job.salary_raw],
  ].filter(([, value]) => value);
  return (
    <div className="grid gap-6 select-text">
      <header className="grid gap-3">
        <div className="flex justify-between items-start gap-3">
          <div className="min-w-0">
            <h2 className="text-xl font-bold normal-case leading-snug break-words">
              {job.title}
            </h2>
            <p className="flex items-center gap-2 mt-1 text-sm text-[var(--color-ink-mute)]">
              <CompanyLogo company={job.company} jobId={job.id} sourceId={job.source_id} logo={job.company_logo} size="lg" />
              <span>{job.company}</span>
            </p>
          </div>
          <div className="flex items-center gap-1 shrink-0">
          {onToggleFavorite && (
            <FavoriteButton
              favorited={job.favorited}
              onToggle={() => onToggleFavorite(job.id)}
              size="md"
            />
          )}
          {onDelete && (
            <RemoveJobButton
              title={job.title}
              company={job.company}
              onClick={() => onDelete(job)}
            />
          )}
          {onClose && onResetPane && <IconButton data-control-semantic label="Reset pane width" onClick={onResetPane}><Icon name="refresh" /></IconButton>}
          {onClose && (
            <IconButton label="Close job details" data-control-action="job.inspector.close" onClick={onClose}>
              <Icon name="close" />
            </IconButton>
          )}
          </div>
        </div>
        <p className="text-sm">
          {formatJobLocation(job.location, job.workplace_type, job.locations)}
        </p>
        <p className="text-sm font-bold">{formatSalary(job)}</p>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="inverted" data-control-action="job.posting.open" onClick={() => onOpenUrl(job.url)}>
            Open posting
          </Button>
          {tracked ? (
            <label className="text-sm">
              Stage{" "}
              <Select aria-label="Application stage" actionId="job.stage.change" value={job.status}
                onChange={(value) => onUpdateStatus(job.id, value as JobStatus)}
                options={TRACKED_STAGES.map((stage) => ({ value: stage, label: stage }))} />
            </label>
          ) : (
            <>
              <Button onClick={() => onUpdateStatus(job.id, "applied")} data-control-action="job.applied.mark">
                Mark as applied
              </Button>
              <Button
                data-control-action="job.ignored.mark" variant="ghost"
                onClick={() => onUpdateStatus(job.id, "ignored")}
              >
                Ignore
              </Button>
            </>
          )}
        </div>
        {!tracked && job.status === "scraped" && (
          <ActionMenu label="More actions" actionId="job.actions.open" items={[{
            actionId: "job.viewed.mark", label: "Mark viewed",
            onSelect: () => onUpdateStatus(job.id, "viewed"),
          }]} />
        )}
      </header>
      <section className="grid gap-2">
        <h3 className="font-bold">Description</h3>
        <JobDescriptionView
          description={job.description}
          fallback={job.description_text}
          onOpenUrl={onOpenUrl}
        />
      </section>
      <section className="grid gap-2">
        <div className="flex justify-between gap-2">
          <label htmlFor={`notes-${job.id}`} className="font-bold">
            Notes
          </label>
          <span role="status" className="text-sm text-[var(--color-ink-mute)]">
            {noteState}
          </span>
        </div>
        <textarea data-control-action="job.notes.edit"
          id={`notes-${job.id}`}
          rows={4}
          value={notes}
          onChange={(e) => {
            latest.current = e.target.value;
            setNotes(e.target.value);
            setNoteState("Unsaved");
          }}
          className="w-full p-3 text-sm border bg-[var(--color-paper)] resize-y"
          placeholder="Add notes or follow-up details…"
        />
        {noteState === "Failed to save" && (
          <Button onClick={() => save(notes)} data-control-action="job.notes.retry">Retry saving notes</Button>
        )}
      </section>
      <details className="border-t pt-4">
        <summary data-control-action="job.details.expand" className="font-bold cursor-pointer min-h-8">
          More details
        </summary>
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm mt-3">
          {Object.entries({
            Status: job.status,
            Role: job.role_type.replaceAll("_", " "),
            Experience: formatExperience(job),
            "New graduate": triStateToLabel(job.is_new_grad),
            Sponsorship: triStateToLabel(job.sponsorship),
            "Work authorization": formatWorkAuth(job),
            Clearance: triStateToLabel(job.clearance),
            Posted: date(job.date_posted),
            "Provider updated": date(job.provider_updated_at),
            Discovered: date(job.date_added),
            "Last seen": date(job.last_seen_at),
            "Stage updated": date(job.status_changed_at),
          }).map(([label, value]) => (
            <div className="contents" key={label}>
              <dt className="text-[var(--color-ink-mute)]">{label}</dt>
              <dd className="break-words">{value}</dd>
            </div>
          ))}
          {job.locations.map((location, i) => (
            <div className="contents" key={i}>
              <dt>Location {i + 1}</dt>
              <dd>
                {[location.city, location.region_code, location.country_code]
                  .filter(Boolean)
                  .join(", ")}
              </dd>
            </div>
          ))}
        </dl>
      </details>
      <details className="border-t pt-4">
        <summary data-control-action="job.evidence.expand" className="font-bold cursor-pointer min-h-8">
          Source evidence
        </summary>
        <div className="grid gap-3 mt-3 text-sm break-words">
          <p>{job.url}</p>
          <p>Provider ID: {job.external_id || "Unknown"}</p>
          {evidence.map(([label, value]) => (
            <div key={label}>
              <h4 className="font-bold">{label}</h4>
              <p>{value}</p>
            </div>
          ))}
          {job.compensation?.map((range, i) => (
            <p key={i}>
              {range.raw || "No raw compensation evidence"} · {range.source}
            </p>
          ))}
        </div>
      </details>
      <details
        className="border-t pt-4"
        onToggle={(e) => setHistoryOpen(e.currentTarget.open)}
      >
        <summary data-control-action="job.history.expand" className="font-bold cursor-pointer min-h-8">
          Activity history
        </summary>
        <div className="grid gap-2 mt-3 text-sm">
          {historyError ? (
            <p role="alert">{historyError}</p>
          ) : events.length ? (
            events.map((event) => (
              <p key={String(event.id)}>
                {event.from_status || "New"} → {event.to_status} ·{" "}
                {date(event.changed_at)}
              </p>
            ))
          ) : (
            <p>No history recorded.</p>
          )}
        </div>
      </details>
    </div>
  );
}
