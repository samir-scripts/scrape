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
  const [step, setStep] = useState(0);
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
      setStep(0);
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
      setStep(1);
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
      width="max-w-2xl"
    >
      <div ref={content} className="grid gap-4 text-sm">
        {isDemoMode() && (
          <p className="border p-2">
            Demonstration: previews contain fictional fixtures. Desktop previews
            extract actual source postings.
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
                onClick={() => {
                  setBusy(true);
                  api
                    .runSourceNow(saved.id)
                    .then(close)
                    .catch((e) => setError(String(e)))
                    .finally(() => setBusy(false));
                }}
                disabled={busy}
              >
                Run now
              </Button>
              <Button onClick={close}>Done</Button>
            </div>
          </>
        ) : (
          <>
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
                  const text = await file.text();
                  const doc = parseDocument(text);
                  if (doc.errors.length)
                    throw new Error(
                      doc.errors.map((e) => e.message).join("; "),
                    );
                  change(await api.validateWorkflow(doc.toJSON()));
                  setAdvanced(true);
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
                  const link = document.createElement("a");
                  const url = URL.createObjectURL(
                    new Blob([workflowSpecToYaml(draft)], {
                      type: "text/yaml",
                    }),
                  );
                  link.href = url;
                  link.download = "workflow.yaml";
                  link.click();
                  URL.revokeObjectURL(url);
                }}
              >
                Export YAML
              </Button>
              <Button onClick={() => setAdvanced(!advanced)}>
                {advanced ? "Simple" : "Advanced"}
              </Button>
              {advanced && (
                <Button
                  onClick={() =>
                    yaml == null
                      ? setYaml(workflowSpecToYaml(draft))
                      : void applyYaml()
                  }
                >
                  {yaml == null ? "Edit YAML" : "Apply YAML to form"}
                </Button>
              )}
            </div>
            {!advanced && (
              <>
                <nav
                  aria-label="Workflow steps"
                  className="flex flex-wrap gap-2"
                >
                  {["1 Source", "2 Matches", "3 Schedule and save"].map(
                    (label, i) => (
                      <Button
                        key={label}
                        aria-current={step === i ? "step" : undefined}
                        onClick={() => setStep(i)}
                      >
                        {label}
                      </Button>
                    ),
                  )}
                </nav>
                {advancedSettings.length > 0 && (
                  <p>
                    Preserved: {advancedSettings.join(", ")}.{" "}
                    <button
                      type="button"
                      className="underline cursor-pointer"
                      onClick={() => setAdvanced(true)}
                    >
                      Review Advanced settings
                    </button>
                  </p>
                )}
              </>
            )}
            {yaml != null ? (
              <label className="grid gap-2">
                Workflow YAML
                <textarea
                  aria-label="Workflow YAML"
                  className="font-mono border bg-[var(--color-paper)] p-2 min-h-80"
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
                {(advanced
                  ? (["Source", "Matching", "Schedule", "Retention"] as const)
                  : ([
                      step === 0
                        ? "Source"
                        : step === 1
                          ? "Matching"
                          : "Schedule",
                    ] as const)
                ).map((section) => (
                  <section className="grid gap-3 border-t pt-3" key={section}>
                    <h3 className="font-mono font-bold uppercase">{section}</h3>
                    <WorkflowSections
                      draft={draft}
                      change={change}
                      advanced={advanced}
                      section={section}
                    />
                  </section>
                ))}
                {(advanced || step === 0) && (
                  <Button
                    disabled={busy || !draft.extractor.url}
                    onClick={() => void detect()}
                  >
                    Detect source
                  </Button>
                )}
                {(advanced || step > 0) && (
                  <div className="flex gap-2">
                    <Button disabled={busy} onClick={() => void runPreview()}>
                      {preview ? "Refresh preview" : "Preview matches"}
                    </Button>
                    {busy && <Button onClick={cancel}>Cancel preview</Button>}
                  </div>
                )}
                {preview && (
                  <section
                    aria-label="Preview results"
                    className="grid gap-2 border-t pt-3"
                  >
                    <p>
                      {preview.fetched} fetched · {preview.matched} matched ·{" "}
                      {preview.rejected} rejected · {preview.blocked} blocked ·{" "}
                      {preview.invalid} invalid
                    </p>
                    {preview.errors.map((e, i) => (
                      <p key={i} className="text-[var(--color-danger)]">
                        {e}
                      </p>
                    ))}
                    {preview.jobs.length === 0 && (
                      <p>
                        No eligible postings. The source may be empty, your
                        matching rules may exclude everything, or postings may
                        be blocked. You may save this workflow with zero
                        matches.
                      </p>
                    )}
                    {preview.sampled && (
                      <p>
                        Showing the first {preview.jobs.length} eligible
                        postings; counts describe fetched postings. Extraction
                        errors may leave the board incomplete.
                      </p>
                    )}
                    {preview.jobs.map(
                      ({ identity, job }) =>
                        job.type === "job" && (
                          <article key={identity} className="border-b py-2">
                            <a
                              href={job.url}
                              onClick={(event) => {
                                event.preventDefault();
                                openPostingUrl(job.url).catch((error) =>
                                  setError(String(error)),
                                );
                              }}
                              target="_blank"
                              rel="noreferrer"
                              className="underline"
                            >
                              {job.title}
                            </a>
                            <p>
                              {job.company} ·{" "}
                              {job.location || "Location unknown"}
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
                )}
                {!advanced && step === 1 && (
                  <Button disabled={!preview} onClick={() => setStep(2)}>
                    Continue to schedule
                  </Button>
                )}
                {(advanced || step === 2) && (
                  <>
                    <p>
                      {draft.metadata.name || "Unnamed workflow"} ·{" "}
                      {draft.extractor.kind} ·{" "}
                      {draft.schedule?.cadence || "global"} ·{" "}
                      {draft.schedule?.time || ""}{" "}
                      {draft.schedule?.timezone || ""}.{" "}
                      {draft.metadata.enabled === false
                        ? "Disabled."
                        : "Enabled."}
                    </p>
                    <Button
                      variant="inverted"
                      disabled={busy || !preview}
                      onClick={() => void save()}
                    >
                      Save workflow
                    </Button>
                    {!preview && (
                      <p>Preview the current configuration before saving.</p>
                    )}
                  </>
                )}
              </>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}
