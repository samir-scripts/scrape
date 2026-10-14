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
import { formatJobLocation } from "../../lib/jobLocationLabel";

export interface JobInspectorProps {
  job: Job | null;
  onClose?: () => void;
  onUpdateStatus: (id: string, status: JobStatus) => void;
  onUpdateNotes: (id: string, notes: string) => void | Promise<unknown>;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (id: string) => void;
  onDelete?: (job: Job) => void;
}
const date = (value: string | null | undefined) =>
  value ? new Date(value).toLocaleString() : "Unknown";
export function JobInspectorPanel(props: JobInspectorProps) {
  const [narrow, setNarrow] = useState(
    () =>
      typeof matchMedia !== "undefined" &&
      matchMedia("(max-width: 1099px)").matches,
  );
  const priorFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (typeof matchMedia === "undefined") return;
    const query = matchMedia("(max-width: 1099px)");
    const change = () => setNarrow(query.matches);
    query.addEventListener("change", change);
    return () => query.removeEventListener("change", change);
  }, []);
  useEffect(() => {
    priorFocus.current = document.activeElement as HTMLElement;
    return () => {
      if (priorFocus.current?.isConnected) priorFocus.current.focus();
    };
  }, []);
  useEffect(() => {
    const active = document.activeElement as HTMLElement | null;
    if (active && !active.closest('[aria-label="Job details"]')) priorFocus.current = active;
  }, [props.job?.id]);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !document.querySelector("dialog[open]")) {
        event.preventDefault();
        props.onClose?.();
      }
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [props.onClose]);
  if (!props.job) return null;
  const content = (
    <JobInspectorContent key={props.job.id} {...props} job={props.job} />
  );
  return narrow ? (
    <Modal
      isOpen
      title="Job details"
      width="max-w-[640px]"
      onClose={() => props.onClose?.()}
    >
      {content}
    </Modal>
  ) : (
    <aside
      aria-label="Job details"
      className="w-[480px] max-w-[45vw] shrink-0 border-l overflow-y-auto bg-[var(--color-paper)] p-4"
    >
      {content}
    </aside>
  );
}
export function JobInspectorContent({
  job,
  onClose,
  onUpdateStatus,
  onUpdateNotes,
  onOpenUrl,
  onToggleFavorite,
  onDelete,
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
            <p className="mt-1 text-sm text-[var(--color-ink-mute)]">
              {job.company}
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
          {onClose && (
            <IconButton label="Close job details" onClick={onClose}>
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
          <Button variant="inverted" onClick={() => onOpenUrl(job.url)}>
            Open posting
          </Button>
          {tracked ? (
            <label className="text-sm">
              Stage{" "}
              <select
                aria-label="Application stage"
                className="border min-h-9 p-2 bg-[var(--color-paper)]"
                value={job.status}
                onChange={(e) =>
                  onUpdateStatus(job.id, e.target.value as JobStatus)
                }
              >
                {TRACKED_STAGES.map((stage) => (
                  <option key={stage} value={stage}>
                    {stage}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <>
              <Button onClick={() => onUpdateStatus(job.id, "applied")}>
                Mark as applied
              </Button>
              <Button
                variant="ghost"
                onClick={() => onUpdateStatus(job.id, "ignored")}
              >
                Ignore
              </Button>
            </>
          )}
        </div>
        {!tracked && job.status === "scraped" && (
          <details>
            <summary className="text-sm cursor-pointer">More actions</summary>
            <Button
              variant="ghost"
              onClick={() => onUpdateStatus(job.id, "viewed")}
            >
              Mark viewed
            </Button>
          </details>
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
        <textarea
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
          <Button onClick={() => save(notes)}>Retry saving notes</Button>
        )}
      </section>
      <details className="border-t pt-4">
        <summary className="font-bold cursor-pointer min-h-8">
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
        <summary className="font-bold cursor-pointer min-h-8">
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
        <summary className="font-bold cursor-pointer min-h-8">
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
