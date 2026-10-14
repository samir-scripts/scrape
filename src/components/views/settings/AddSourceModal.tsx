import { openPostingUrl } from "../../../lib/postingUrl";
import { useEffect, useRef, useState } from "react";
import { parseDocument } from "yaml";
import type { Source } from "../../../bindings/Source";
import type { Settings } from "../../../bindings/Settings";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../../../bindings/WorkflowPreview";
import { api } from "../../../lib/api";
import {
  sourceToWorkflowSpec,
  workflowSpecToYaml,
  canonicalizeAtsUrl,
} from "../../../lib/yamlWorkflow";
import { isDemoMode } from "../../../lib/runtime";
import { Modal } from "../../ui/Modal";
import { Button } from "../../ui/Button";
import { WorkflowSections } from "./WorkflowSections";

interface AddSourceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSourceSaved: () => void;
  initialSource?: Source | null;
  settings?: Settings;
}
const newDraft = (): WorkflowSpec => ({
  version: 3,
  metadata: { name: "", enabled: true },
  extractor: { kind: "jsonld", url: "" },
  schedule: {
    cadence: "daily",
    time: "09:00",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  },
  filters: {},
  retention: { days: 30, policy: "untracked_only" },
});
export function AddSourceModal({
  isOpen,
  onClose,
  onSourceSaved,
  initialSource,
}: AddSourceModalProps) {
  const [draft, setDraft] = useState<WorkflowSpec>(newDraft);
  const [advanced, setAdvanced] = useState(false);
  const [yaml, setYaml] = useState<string | null>(null);
  const [detected, setDetected] = useState("");
  const [preview, setPreview] = useState<WorkflowPreview | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState<Source | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const revision = useRef(0);
  const request = useRef<string | null>(null);
  const cancel = () => {
    if (request.current)
      void api.cancelWorkflowPreview(request.current).catch(() => {});
    request.current = null;
    revision.current++;
    setBusy(false);
  };
  useEffect(() => {
    if (isOpen) {
      cancel();
      setDraft(
        initialSource ? sourceToWorkflowSpec(initialSource) : newDraft(),
      );
      setAdvanced(false);
      setYaml(null);
      setDetected(initialSource?.identifier || "");
      setPreview(null);
      setError("");
      setSaved(null);
    }
    return () => {
      if (request.current)
        void api.cancelWorkflowPreview(request.current).catch(() => {});
      request.current = null;
      revision.current++;
    };
  }, [isOpen, initialSource]);
  const change = (next: WorkflowSpec) => {
    cancel();
    setDraft(next);
    setPreview(null);
    setError("");
    if (next.extractor.url !== draft.extractor.url) setDetected("");
  };
  const close = () => {
    cancel();
    onClose();
  };
  const detect = async () => {
    const current = ++revision.current;
    setBusy(true);
    setError("");
    setPreview(null);
    try {
      const result = await api.detectSource(draft.extractor.url);
      if (revision.current !== current) return;
      setDraft({
        ...draft,
        metadata: {
          ...draft.metadata,
          name: draft.metadata.name || result.name,
          company: draft.metadata.company || result.name,
        },
        extractor: {
          ...draft.extractor,
          kind: result.kind as WorkflowSpec["extractor"]["kind"],
          url: canonicalizeAtsUrl(result.kind, result.identifier),
        },
      });
      setDetected(canonicalizeAtsUrl(result.kind, result.identifier));
    } catch (e) {
      if (revision.current === current)
        setError(
          `${String(e)} Open Advanced to configure a supported board or selectors.`,
        );
    } finally {
      if (revision.current === current) setBusy(false);
    }
  };
  const applyYaml = async () => {
    try {
      const doc = parseDocument(yaml || "");
      if (doc.errors.length)
        throw new Error(doc.errors.map((e) => e.message).join("; "));
      const validated = await api.validateWorkflow(doc.toJSON());
      change(validated);
      setYaml(null);
    } catch (e) {
      setError(String(e));
    }
  };
  const runPreview = async () => {
    if (content.current?.querySelector("[aria-invalid=true]")) {
      setError("Fix the highlighted inputs before previewing.");
      return;
    }
    const current = ++revision.current;
    const id = crypto.randomUUID();
    request.current = id;
    setBusy(true);
    setError("");
    setPreview(null);
    try {
      const result = await api.previewWorkflow(draft, id);
      if (revision.current === current) {
        setPreview(result);
        setDraft(result.spec);
      }
    } catch (e) {
      if (revision.current === current) setError(String(e));
    } finally {
      if (revision.current === current) {
        request.current = null;
        setBusy(false);
      }
    }
  };
  const save = async () => {
    if (!preview || preview.errors.length || busy) return;
    setBusy(true);
    setError("");
    try {
      const source = await api.saveWorkflow(draft, initialSource?.id);
      setSaved(source);
      onSourceSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const rules = draft.filters?.matching;
  const advancedSettings = [
    draft.filters?.query || draft.filters?.policy ? "expert rules" : null,
    draft.filters?.include_keywords?.length ||
    draft.filters?.exclude_keywords?.length
      ? "imported keyword rules"
      : null,
    draft.filters?.role_types?.length || rules?.role_types?.length
      ? "role restrictions"
      : null,
    rules?.compensation || draft.filters?.compensation
      ? "native pay rules"
      : null,
    rules?.posting_date ? "posting dates" : null,
    rules?.workplaces?.length ||
    rules?.experience_min != null ||
    rules?.experience_max != null ||
    rules?.sponsorship?.length ||
    rules?.work_auth?.length ||
    rules?.clearance?.length ||
    rules?.countries?.length ||
    rules?.regions?.length ||
    (rules?.new_grad && rules.new_grad !== "any")
      ? "additional matching rules"
      : null,
    rules?.keywords?.all.length ||
    rules?.keywords?.exclude.length ||
    rules?.keywords?.fields.some((field) => field !== "title")
      ? "additional keyword rules"
      : null,
    draft.extractor.kind === "selector" ||
    Object.keys(draft.extractor.source_config || {}).some(
      (key) => key !== "url",
    )
      ? "custom extraction"
      : null,
    draft.schedule?.weekdays?.length ||
    (draft.schedule?.cadence &&
      !["daily", "manual"].includes(draft.schedule.cadence))
      ? "advanced schedule"
      : null,
    draft.retention?.days != null && draft.retention.days !== 30
      ? "custom retention"
      : null,
  ].filter(Boolean);
  return (
    <Modal
      isOpen={isOpen}
      onClose={close}
      title={initialSource ? "Edit workflow" : "Create workflow"}
      width="max-w-[800px]"
      footer={
        !saved && (
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Button onClick={close}>Cancel</Button>
            <div className="flex flex-wrap gap-2">
              {busy && <Button onClick={cancel}>Cancel preview</Button>}
              {yaml == null && (
                <Button
                  variant={preview ? "default" : "inverted"}
                  disabled={busy || !draft.extractor.url}
                  onClick={() => void runPreview()}
                >
                  {preview ? "Refresh preview" : "Preview matches"}
                </Button>
              )}
              <Button
                variant="inverted"
                disabled={
                  busy || !preview || !!preview.errors.length || yaml != null
                }
                onClick={() => void save()}
              >
                Save workflow
              </Button>
            </div>
          </div>
        )
      }
    >
      <div ref={content} className="grid gap-6 text-sm">
        {isDemoMode() && (
          <p className="text-[var(--color-ink-mute)]">
            Example mode. Open the desktop application to test a live source.
          </p>
        )}
        {error && (
          <p role="alert" className="text-[var(--color-danger)]">
            {error}
          </p>
        )}
        {saved ? (
          <>
            <p>Saved {saved.name}. Saving has not started a scrape.</p>
            <p>
              Next run:{" "}
              {saved.next_run_at
                ? new Date(saved.next_run_at).toLocaleString()
                : saved.cadence === "manual"
                  ? "Manual only"
                  : "Calculated by the desktop scheduler"}
              .
            </p>
            <div className="flex gap-2">
              <Button
                variant="inverted"
                disabled={busy}
                onClick={() => {
                  setBusy(true);
                  api
                    .runSourceNow(saved.id)
                    .then(close)
                    .catch((e) => setError(String(e)))
                    .finally(() => setBusy(false));
                }}
              >
                Run now
              </Button>
              <Button onClick={close}>Done</Button>
            </div>
          </>
        ) : (
          <>
            {yaml != null ? (
              <label className="grid gap-2">
                Workflow YAML
                <textarea
                  aria-label="Workflow YAML"
                  className="font-code border bg-[var(--color-paper)] p-3 min-h-80"
                  value={yaml}
                  onChange={(e) => {
                    cancel();
                    setPreview(null);
                    setYaml(e.target.value);
                  }}
                />
              </label>
            ) : (
              <>
                {(["Source", "Matching", "Schedule"] as const).map(
                  (section) => (
                    <section className="grid gap-3" key={section}>
                      <h3 className="text-base font-bold normal-case">
                        {section === "Matching"
                          ? "Matching (optional)"
                          : section}
                      </h3>
                      <WorkflowSections
                        draft={draft}
                        change={change}
                        advanced={advanced}
                        section={section}
                        onSourceBlur={() => {
                          if (draft.extractor.url && !detected && !busy)
                            void detect();
                        }}
                      />
                      {section === "Source" && (
                        <div
                          className="text-sm text-[var(--color-ink-mute)]"
                          aria-live="polite"
                        >
                          {busy && !request.current
                            ? "Detecting source…"
                            : detected
                              ? `Source: ${draft.extractor.kind} · ${draft.metadata.company || draft.metadata.name}`
                              : "Paste a careers URL, then leave the field to detect the source."}
                          {error && !detected && (
                            <Button
                              size="sm"
                              onClick={() => void detect()}
                              disabled={busy}
                            >
                              Retry detection
                            </Button>
                          )}
                        </div>
                      )}
                    </section>
                  ),
                )}
                <p className="text-[var(--color-ink-mute)]">
                  {draft.schedule?.cadence === "manual"
                    ? "Runs only when you choose Run now."
                    : `${draft.schedule?.cadence || "Inherited schedule"} ${draft.schedule?.time || ""} · ${draft.schedule?.timezone || "Global timezone"}`}
                  . Retention: {draft.retention?.days ?? "inherited"} days;
                  protected jobs are kept.
                </p>
              </>
            )}
            <section className="grid gap-3 border-t pt-4">
              <button
                type="button"
                aria-expanded={advanced}
                className="text-left underline cursor-pointer min-h-9"
                onClick={() => setAdvanced(!advanced)}
              >
                {advanced ? "Hide advanced settings" : "Advanced settings"}
              </button>
              {advancedSettings.length > 0 && (
                <p className="text-[var(--color-ink-mute)]">
                  Active advanced settings: {advancedSettings.join(", ")}.
                </p>
              )}
              {advanced && (
                <>
                  <p className="text-[var(--color-ink-mute)]">
                    Absent fields inherit global defaults. Explicit empty values
                    override them. YAML exposes every supported setting.
                  </p>
                  {yaml == null && (
                    <WorkflowSections
                      draft={draft}
                      change={change}
                      advanced
                      section="Retention"
                    />
                  )}
                  <input
                    ref={fileInput}
                    type="file"
                    accept=".yaml,.yml"
                    aria-label="Workflow YAML file"
                    className="hidden"
                    onChange={async (e) => {
                      const file = e.target.files?.[0];
                      if (!file) return;
                      try {
                        const doc = parseDocument(await file.text());
                        if (doc.errors.length)
                          throw new Error(
                            doc.errors.map((e) => e.message).join("; "),
                          );
                        change(await api.validateWorkflow(doc.toJSON()));
                        setYaml(null);
                      } catch (e) {
                        setError(String(e));
                      } finally {
                        if (fileInput.current) fileInput.current.value = "";
                      }
                    }}
                  />
                  <div className="flex flex-wrap gap-2">
                    <Button
                      aria-label="Import YAML"
                      onClick={() => fileInput.current?.click()}
                    >
                      Import YAML
                    </Button>
                    <Button
                      onClick={() => {
                        const url = URL.createObjectURL(
                          new Blob([workflowSpecToYaml(draft)], {
                            type: "text/yaml",
                          }),
                        );
                        const link = document.createElement("a");
                        link.href = url;
                        link.download = "workflow.yaml";
                        link.click();
                        URL.revokeObjectURL(url);
                      }}
                    >
                      Export YAML
                    </Button>
                    <Button
                      onClick={() =>
                        yaml == null
                          ? setYaml(workflowSpecToYaml(draft))
                          : void applyYaml()
                      }
                    >
                      {yaml == null ? "Edit YAML" : "Apply YAML to form"}
                    </Button>
                  </div>
                </>
              )}
            </section>
            {preview ? (
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
                    {preview.fetched} fetched · {preview.matched} matched ·{" "}
                    {preview.rejected} rejected · {preview.blocked} blocked ·{" "}
                    {preview.invalid} invalid
                  </p>
                </details>
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
                    Showing a sample of {preview.jobs.length} eligible postings.
                    Counts describe fetched postings; extraction errors may
                    leave the board incomplete.
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
            )}
          </>
        )}
      </div>
    </Modal>
  );
}
