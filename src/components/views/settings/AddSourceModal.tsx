import { INGESTION_LIMITS } from "../../../bindings/ingestionLimits";
import { useLayoutEffect, useRef } from "react";
import { api } from "../../../lib/api";
import type { Source } from "../../../bindings/Source";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import { useWorkflowDraft, WORKFLOW_STEPS } from "../../../hooks/useWorkflowDraft";
import { workflowSpecToYaml } from "../../../lib/yamlWorkflow";
import { isDemoMode, isRecordedMode } from "../../../lib/runtime";
import { Modal } from "../../ui/Modal";
import { Button } from "../../ui/Button";
import { WorkflowSections } from "./WorkflowSections";
import { WorkflowImportButton } from "./WorkflowImportButton";
import { WorkflowPreviewResults } from "./WorkflowPreviewResults";

interface AddSourceModalProps {
  isOpen: boolean; onClose: () => void; onSourceSaved: () => void;
  initialSource?: Source | null; initialSpec?: WorkflowSpec | null;
}
export function AddSourceModal({ isOpen, onClose, onSourceSaved, initialSource, initialSpec }: AddSourceModalProps) {
  const model = useWorkflowDraft({ isOpen, initialSource, initialSpec, onSourceSaved });
  const { draft, change, step, advanced, setAdvanced, yaml, setYaml, detected, preview, error, setError, busy, saved } = model;
  const persisting = model.operation === "save" || model.operation === "run";
  const locked = persisting || !!model.pendingImport;
  const hadImport = useRef(false);
  useLayoutEffect(() => {
    if (hadImport.current && !model.pendingImport) model.content.current?.querySelector<HTMLElement>('button[aria-label="Import workflow YAML"]')?.focus({ preventScroll: true });
    hadImport.current = !!model.pendingImport;
  }, [model.pendingImport]);
  const close = () => { if (model.pendingImport) { model.cancelImport(); return; } if (!persisting) { model.cancel(); onClose(); } };
  const index = WORKFLOW_STEPS.indexOf(step);
  const rules = draft.filters?.matching;
  const advancedSettings = [
    draft.filters?.query || draft.filters?.policy ? "expert rules" : null,
    draft.filters?.locations?.length ? "additional location text" : null,
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
    rules?.experience_min != null ||
    rules?.experience_max != null ||
    rules?.sponsorship?.length ||
    rules?.work_auth?.length ||
    rules?.clearance?.length ||
    (rules?.new_grad && rules.new_grad !== "any")
      ? "additional matching rules"
      : null,
    rules?.keywords?.all?.length ||
    rules?.keywords?.exclude?.length ||
    rules?.keywords?.fields?.some((field) => field !== "title")
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
  return <Modal isOpen={isOpen} onClose={close} title={initialSource ? "Edit workflow" : "Create workflow"} width="max-w-[800px]"
    footer={!saved && <div className="flex flex-wrap items-center justify-between gap-3">
      <Button onClick={close} disabled={locked}>Cancel</Button>
      <div className="flex flex-wrap gap-2">
        {model.operation === "preview" && <Button onClick={model.cancel}>Cancel preview</Button>}
        {index > 0 && <Button disabled={busy || !!model.pendingImport} onClick={() => model.navigate(WORKFLOW_STEPS[index - 1])}>Back</Button>}
        {yaml == null && (step === "Matching" || step === "Review") && <Button disabled={busy || !!model.pendingImport || isRecordedMode()} onClick={() => void model.runPreview()}>{preview ? "Refresh preview" : "Preview matches"}</Button>}
        {step !== "Review" ? <Button variant="inverted" disabled={busy || !!model.pendingImport || yaml != null} onClick={() => void model.next()}>{step === "Schedule" ? "Review workflow" : `Continue to ${WORKFLOW_STEPS[index + 1]}`}</Button>
          : <Button variant="inverted" disabled={!model.canSave} onClick={() => void model.save()}>Save workflow</Button>}
      </div>
    </div>}>
    <div ref={model.content} className="grid gap-5 text-sm">
      {isDemoMode() && <p className="text-[var(--color-ink-mute)]">Example mode. Open the desktop application to test a live source.</p>}
      {error && <div role="alert" className="text-[var(--color-danger)]"><p>{error}</p><button type="button" className="underline min-h-8" onClick={() => { model.navigate(model.errorStep); requestAnimationFrame(() => model.content.current?.querySelector<HTMLElement>(`[data-workflow-step="${model.errorStep}"] input`)?.focus()); }}>Go to {model.errorStep}</button></div>}
      {saved ? <>
        <p>Saved {saved.name}. Saving has not started a scrape.</p>
        <p>Next run: {saved.next_run_at ? new Date(saved.next_run_at).toLocaleString() : saved.cadence === "manual" ? "Manual only" : "Calculated by the desktop scheduler"}.</p>
        <div className="flex gap-2"><Button variant="inverted" disabled={busy || !!model.pendingImport || isRecordedMode()} onClick={async () => { if (await model.runSaved()) onClose(); }}>Run now</Button>{model.operation === "run" && <Button variant="danger" onClick={() => void api.cancelScrape().catch(error => setError(String(error)))}>Cancel run</Button>}<Button disabled={busy} onClick={close}>Done</Button></div>
      </> : <>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <nav aria-label="Workflow steps"><ol className="flex flex-wrap items-center gap-2">
            {WORKFLOW_STEPS.map((value, position) => <li key={value} className="flex items-center gap-2">
              {position > 0 && <span aria-hidden="true">→</span>}
              <button type="button" aria-current={step === value ? "step" : undefined} disabled={position > model.visited || busy || !!model.pendingImport}
                className={`min-h-9 px-2 border ${step === value ? "bg-[var(--color-ink)] text-[var(--color-paper)]" : "border-transparent"}`}
                onClick={() => model.navigate(value)}>{value}</button>
            </li>)}
          </ol></nav>
          <WorkflowImportButton disabled={!isOpen || locked || isRecordedMode()} onImport={model.replaceImport} />
        </div>
        {model.pendingImport && <section role="region" aria-label="Replace unsaved workflow changes" className="border border-[var(--color-warning)] p-3 grid gap-3">
          <p>Replace unsaved changes with {model.pendingImport.metadata.name}? Your current draft stays unchanged until you replace it.</p>
          <div className="flex flex-wrap gap-2"><Button autoFocus onClick={model.cancelImport}>Keep editing</Button><Button variant="inverted" onClick={model.confirmImport}>Replace draft</Button></div>
        </section>}
        {yaml != null && <label className="grid gap-2">Workflow YAML<textarea disabled={locked} aria-label="Workflow YAML" className="font-code border bg-[var(--color-paper)] p-3 min-h-80" value={yaml} onChange={event => setYaml(event.target.value)} /></label>}
        <fieldset disabled={locked} className={yaml != null ? "hidden" : "min-w-0 grid gap-4"}>
          {(["Source", "Matching", "Schedule"] as const).map(section => <section key={section} data-workflow-step={section} hidden={step !== section} className={step === section ? "grid gap-3" : "hidden"}>
            <h3 className="text-base font-bold normal-case">{section === "Matching" ? "Matching (optional)" : section}</h3>
            <WorkflowSections draft={draft} change={change} advanced={advanced} section={section} />
            {section === "Source" && <p aria-live="polite" className="text-[var(--color-ink-mute)]">{model.operation === "detect" ? "Detecting source…" : detected ? `Source: ${draft.extractor.kind} · ${draft.metadata.company || draft.metadata.name}` : "Paste a careers URL. Continue to detect its source, or configure an adapter in Advanced settings."}</p>}
            {section === "Schedule" && <details><summary className="cursor-pointer min-h-9">Retention</summary><WorkflowSections draft={draft} change={change} advanced section="Retention" /></details>}
          </section>)}
        </fieldset>
        {step === "Review" && yaml == null && <section data-workflow-step="Review" className="grid gap-3">
          <h3 className="font-bold text-base">Review {draft.metadata.name}</h3>
          <p>Maximum {draft.max_jobs_per_run ?? INGESTION_LIMITS.default} matching jobs per run; preview accepts at most {Math.min(draft.max_jobs_per_run ?? INGESTION_LIMITS.default, INGESTION_LIMITS.preview)}.</p>
          <p className="break-words">{draft.extractor.kind} · {draft.extractor.url}</p>
          <p>{draft.metadata.enabled === false ? "Disabled" : "Enabled"} · {draft.schedule?.cadence || "daily"} {draft.schedule?.time || ""} · {draft.schedule?.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone}</p>
          <p>Retention: {draft.retention?.days ?? 30} days. Tracked jobs, notes and favorites are protected.</p>
          <details><summary className="cursor-pointer min-h-9">All configured rules</summary><pre className="font-code whitespace-pre-wrap break-words text-xs">{workflowSpecToYaml(draft)}</pre></details>
          <p>Saving creates or updates the workflow. It does not run a scrape.</p>
        </section>}
        <section className="grid gap-3 border-t pt-3">
          <button type="button" aria-expanded={advanced} className="text-left underline cursor-pointer min-h-9" onClick={() => setAdvanced(!advanced)}>{advanced ? "Hide advanced settings" : "Advanced settings"}</button>
          {advancedSettings.length > 0 && <p className="text-[var(--color-ink-mute)]">Active advanced settings: {advancedSettings.join(", ")}.</p>}
          {advanced && <div className="flex flex-wrap gap-2"><Button disabled={locked} onClick={() => yaml == null ? setYaml(workflowSpecToYaml(draft)) : void model.applyYaml()}>{yaml == null ? "Edit YAML" : "Apply YAML to form"}</Button><Button disabled={yaml != null} onClick={() => { const url = URL.createObjectURL(new Blob([workflowSpecToYaml(draft)], { type: "text/yaml" })); const link = document.createElement("a"); link.href = url; link.download = "workflow.yaml"; link.click(); URL.revokeObjectURL(url); }}>Export YAML</Button></div>}
        </section>
        {(step === "Matching" || step === "Review") && <WorkflowPreviewResults preview={preview} draft={draft} setError={setError} />}
      </>}
    </div>
  </Modal>;
}
