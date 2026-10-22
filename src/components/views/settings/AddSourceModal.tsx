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
import { Icon } from "../../ui/Icon";
import { WorkflowSections } from "./WorkflowSections";
import { WorkflowImportButton } from "./WorkflowImportButton";
import { WorkflowPreviewResults } from "./WorkflowPreviewResults";
import { Select } from "../../ui/Select";
import { SAMPLE_WORKFLOW_TEMPLATES } from "../../../lib/yamlWorkflow";

interface AddSourceModalProps {
  isOpen: boolean; onClose: () => void; onSourceSaved: () => void;
  initialSource?: Source | null; initialSpec?: WorkflowSpec | null;
}
export function AddSourceModal({ isOpen, onClose, onSourceSaved, initialSource, initialSpec }: AddSourceModalProps) {
  const model = useWorkflowDraft({ isOpen, initialSource, initialSpec, onSourceSaved });
  const { draft, change, step, advanced, setAdvanced, yaml, setYaml, preview, error, setError, busy, saved } = model;
  const persisting = model.operation === "save" || model.operation === "run";
  const locked = persisting || !!model.pendingImport || !!model.pendingCompany;
  const hadImport = useRef(false);
  useLayoutEffect(() => {
    if (hadImport.current && !model.pendingImport) model.content.current?.querySelector<HTMLElement>('button[aria-label="Import workflow YAML"]')?.focus({ preventScroll: true });
    hadImport.current = !!model.pendingImport;
  }, [model.pendingImport]);
  const close = () => { if (model.pendingCompany) { model.cancelCompany(); return; } if (model.pendingImport) { model.cancelImport(); return; } if (!persisting) { model.cancel(); onClose(); } };
  const index = WORKFLOW_STEPS.indexOf(step);
  const rules = draft.filters?.matching;
  const hasScripts = draft.sources.some(source => source.extractor.script || source.extractor.mode === "script");
  const stepHelp: Record<string, string> = {
    Roles: "Choose job titles, employers, and keywords.",
    Criteria: "Set location, recency, experience, and employment.",
    Sources: "Review search feeds and direct career boards.",
    Schedule: "Set when this workflow runs.",
    Review: "Check the configuration and preview before saving.",
  };
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
    rules?.workplaces?.length ? `workplace: ${rules.workplaces.join(", ")}` : null,
    rules?.countries?.length ? `countries: ${rules.countries.join(", ")}` : null,
    rules?.regions?.length ? `regions: ${rules.regions.join(", ")}` : null,
    rules?.cities?.length ? `cities: ${rules.cities.join(", ")}` : null,
    rules?.experience_min != null ||
    rules?.experience_max != null ||
    rules?.sponsorship?.length ||
    rules?.work_auth?.length ||
    rules?.clearance?.length ||
    (rules?.new_grad && rules.new_grad !== "any")
      ? "additional matching rules"
      : null,
    draft.sources?.some(
      (s) =>
        s.extractor.kind === "selector" ||
        s.extractor.mode != null ||
        s.extractor.script != null ||
        s.extractor.session_id != null ||
        Object.keys(s.extractor.source_config || {}).some(
          (key) => key !== "url",
        ),
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
  return <Modal isOpen={isOpen} onClose={close} title={initialSource ? "Edit workflow" : "Create workflow"} width="w-[1200px] max-w-[calc(100vw-16px)]" wideOnMobile
    footer={!saved && <div className="flex flex-wrap items-center justify-between gap-3">
      <Button onClick={close} disabled={locked}>Cancel</Button>
      <div className="flex flex-wrap gap-2">
        {model.operation === "preview" && <Button onClick={model.cancel}>Cancel preview</Button>}
        {hasScripts && step !== "Review" && <Button disabled={busy || locked || yaml != null} onClick={() => void model.saveDisabled()}>Save disabled for review</Button>}
        {index > 0 && <Button disabled={busy || locked} onClick={() => model.navigate(WORKFLOW_STEPS[index - 1])}>Back</Button>}
        {yaml == null && step === "Review" && <Button disabled={busy || locked || isRecordedMode()} onClick={() => void model.runPreview()}>{preview ? "Refresh preview" : "Preview matches"}</Button>}
        {step !== "Review" ? <Button variant="inverted" className="transition-none" disabled={busy || locked || yaml != null} onClick={() => void model.next()}>{`Continue to ${WORKFLOW_STEPS[index + 1]}`}</Button>
          : <Button variant="inverted" disabled={!model.canSave} onClick={() => void model.save()}>Save workflow</Button>}
      </div>
    </div>}>
    <div ref={model.content} className="grid gap-5 text-sm">
      {!saved && <div role="group" aria-label="Workflow mode" className="flex w-fit border border-[var(--color-border)] p-1">
        {(["Simple", "Advanced"] as const).map((mode, index) => {
          const selected = advanced === (mode === "Advanced");
          return <button key={mode} type="button" aria-pressed={selected} onClick={() => { model.commitInputs(); setAdvanced(mode === "Advanced"); }}
            className={`min-h-9 px-4 text-xs font-semibold uppercase ${selected ? "bg-[var(--color-ink)] text-[var(--color-paper)]" : "hover:bg-[var(--color-surface-raised)]"} ${index ? "border-l border-[var(--color-border)]" : ""}`}>
            {mode}
          </button>;
        })}
      </div>}
      {!advanced && advancedSettings.length > 0 && <p className="text-xs text-[var(--color-ink-mute)]">Active advanced settings: {advancedSettings.join(", ")}. <button type="button" className="underline" onClick={() => setAdvanced(true)}>Review Advanced</button></p>}
      {isDemoMode() && <p className="text-[var(--color-ink-mute)]">Example mode. Open the desktop application to test a live source.</p>}
      {error && <div role="alert" className="text-[var(--color-danger)]"><p>{error}</p><button type="button" className="underline min-h-8" onClick={() => { model.navigate(model.errorStep); requestAnimationFrame(() => model.content.current?.querySelector<HTMLElement>(`[data-workflow-step="${model.errorStep}"] input`)?.focus()); }}>Go to {model.errorStep}</button></div>}
      {saved ? <>
        <p>Saved {saved.name}. Saving has not started a scrape.</p>{saved.needs_review && <p>Saved disabled. Reopen this workflow to review and trust each saved script. Trust keeps the workflow disabled until you explicitly enable it.</p>}
        <p>Next run: {!saved.enabled ? "Disabled" : saved.next_run_at ? new Date(saved.next_run_at).toLocaleString() : saved.cadence === "manual" ? "Manual only" : "Calculated by the desktop scheduler"}.</p>
        <div className="flex gap-2"><Button variant="inverted" disabled={busy || !!model.pendingImport || isRecordedMode() || saved.needs_review || !saved.enabled} onClick={async () => { if (await model.runSaved()) onClose(); }}>Run now</Button>{model.operation === "run" && <Button variant="danger" onClick={() => void api.cancelScrape().catch(error => setError(String(error)))}>Cancel run</Button>}<Button disabled={busy} onClick={close}>Done</Button></div>
      </> : <>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <nav aria-label="Workflow steps" className="min-w-0 flex-1"><ol className="grid grid-cols-2 sm:grid-cols-5 gap-2">
            {WORKFLOW_STEPS.map((value, position) => <li key={value} className="min-w-0">
              <button type="button" aria-label={value} aria-current={step === value ? "step" : undefined} disabled={position > model.visited || busy || locked}
                className={`w-full min-h-10 flex items-center gap-2 border px-2 text-xs font-semibold uppercase disabled:opacity-50 ${step === value ? "bg-[var(--color-text)] text-[var(--color-surface)]" : "border-[var(--color-border)] hover:bg-[var(--color-surface-raised)]"}`}
                onClick={() => model.navigate(value)}>
                <span aria-hidden="true" className="inline-flex items-center justify-center w-5 h-5 shrink-0 border border-current">
                  {position < index ? <Icon name="check" size="sm" /> : String(position + 1).padStart(2, "0")}
                </span>{value}
              </button>
            </li>)}
          </ol></nav>
          <WorkflowImportButton disabled={!isOpen || locked || isRecordedMode()} onImport={model.replaceImport} />
        </div>
        <p className="text-[var(--color-text-muted)]">{stepHelp[step]}</p>
        {model.pendingImport && <section role="region" aria-label="Replace unsaved workflow changes" className="border border-[var(--color-warning)] p-3 grid gap-3">
          <p>Replace unsaved changes with {model.pendingImport.metadata.name}? Your current draft stays unchanged until you replace it.</p>
          <div className="flex flex-wrap gap-2"><Button autoFocus onClick={model.cancelImport}>Keep editing</Button><Button variant="inverted" onClick={model.confirmImport}>Replace draft</Button></div>
        </section>}
        {model.pendingCompany && <section role="region" aria-label="Replace source" className="border border-[var(--color-warning)] p-3 grid gap-3">
          <p>Replace this source with the selected company? Other sources, matching rules, schedule and workflow name stay unchanged.</p>
          <div className="flex flex-wrap gap-2"><Button autoFocus onClick={model.cancelCompany}>Keep editing</Button><Button variant="inverted" onClick={model.confirmCompany}>Replace source</Button></div>
        </section>}
        {yaml != null && <label className="grid gap-2">Workflow YAML<textarea disabled={locked} aria-label="Workflow YAML" className="font-code border bg-[var(--color-paper)] p-3 min-h-80" value={yaml} onChange={event => setYaml(event.target.value)} /></label>}
        <fieldset disabled={locked} className={yaml != null ? "hidden" : "min-w-0 grid gap-4"}>
          {WORKFLOW_STEPS.filter(section => section !== "Review").map(section => <section key={section} data-workflow-step={section} hidden={step !== section} className={step === section ? "grid gap-3" : "hidden"}>
            <h3 className="text-base font-bold normal-case">{section}</h3>
            <WorkflowSections key={`${section}:${model.inputSession}`} draft={draft} change={change} advanced={advanced} section={section} initialSource={initialSource} onTrustChanged={() => model.invalidate()} selectCompany={model.selectCompany} setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} onDetect={index => void model.detect(index)} preview={preview} />
            {section === "Schedule" && <details><summary className="cursor-pointer min-h-9">Retention</summary><WorkflowSections draft={draft} change={change} advanced section="Retention" initialSource={initialSource} onTrustChanged={() => model.invalidate()} setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} /></details>}
          </section>)}
        </fieldset>
        {step === "Review" && yaml == null && <section className="grid gap-3" data-workflow-step="Review">
          <h3 className="text-base font-bold normal-case">Review</h3>
          <section aria-label="Workflow readiness" aria-live="polite"
            className={`grid gap-2 border-l-4 p-4 bg-[var(--color-surface-raised)] ${model.canSave ? "border-[var(--color-success)]" : "border-[var(--color-warning)]"}`}>
            <h3 className={`font-semibold ${model.canSave ? "text-[var(--color-success)]" : "text-[var(--color-warning)]"}`}>
              {model.operation === "preview" ? "Preview in progress…" : model.canSave ? `Preview complete — ${preview!.matched - preview!.blocked} eligible postings found` : preview?.errors.length ? "Preview needs attention" : "Preview required before saving"}
            </h3>
            <p>{model.canSave ? "Configuration is ready to save." : isDemoMode() ? "Example preview only; no live source is contacted." : "Preview checks the current workflow and does not save jobs."}</p>
            {!model.canSave && <div><Button variant="inverted" disabled={busy || locked || yaml != null || isRecordedMode()} onClick={() => void model.runPreview()}>Run Preview</Button></div>}
            {hasScripts && <>
              <p>Custom scripts require local review and trust. Save disabled, then reopen to review each script; trust does not enable the workflow.</p>
              <div><Button variant="inverted" disabled={busy || locked || yaml != null} onClick={() => void model.saveDisabled()}>Save disabled for review</Button></div>
            </>}
          </section>
          <h3 className="font-bold text-base">Workflow summary</h3>
          <p>Maximum {draft.max_jobs_per_run ?? INGESTION_LIMITS.default} matching jobs per run; preview accepts at most {Math.min(draft.max_jobs_per_run ?? INGESTION_LIMITS.default, INGESTION_LIMITS.preview)}.</p>
          <div className="grid gap-1">
            <span className="font-semibold">Sources ({draft.sources?.length || 0}):</span>
            <ul className="list-disc pl-5">
              {draft.sources?.map((s) => (
                <li key={s.id} className="break-words">
                  {s.label} ({s.extractor.kind}{s.extractor.mode ? ` · ${s.extractor.mode}` : ""}) · {s.extractor.url || s.extractor.script?.path || "Cross-company search"} {s.enabled ? "" : "[Disabled]"}
                </li>
              ))}
            </ul>
          </div>
          <p>{draft.metadata.enabled === false ? "Disabled" : "Enabled"} · {draft.schedule?.cadence || "daily"} {draft.schedule?.time || ""} · {draft.schedule?.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone}</p>
          <p>Retention: {draft.retention?.days ?? 30} days. Tracked jobs, notes and favorites are protected.</p>
          <section aria-label="Matching rules summary" className="grid gap-1">
            <h4 className="font-semibold">Matching rules</h4>
            <dl className="grid gap-2">
              <div><dt className="font-semibold">Target roles</dt><dd>{rules?.target_roles?.length ? rules.target_roles.join(", ") : "Any role"}</dd></div>
              <div><dt className="font-semibold">Companies</dt><dd>{rules?.companies?.length ? rules.companies.join(", ") : "All employers"}</dd></div>
              <div><dt className="font-semibold">Posting window</dt><dd>
                {!rules?.posting_date ? "Any time" : <>
                  {rules.posting_date.preset?.replaceAll("_", " ") ||
                    (rules.posting_date.last_hours ? `Past ${rules.posting_date.last_hours} hours` :
                      `${rules.posting_date.from || "No start bound"} through ${rules.posting_date.through || "No end bound"}`)}
                  {` · ${rules.posting_date.timezone} · ${rules.posting_date.include_unknown ? "Include" : "Exclude"} unknown dates`}
                </>}
              </dd></div>
              <div><dt className="font-semibold">Employment type</dt><dd>{(rules?.role_types?.length ? rules.role_types : draft.filters?.role_types)?.join(", ").replaceAll("_", " ") || "Any, including unknown"}</dd></div>
              <div><dt className="font-semibold">{rules?.experience_match_mode === "advertised_range_overlap" && (rules.experience_min != null || rules.experience_max != null) ? "Advertised range overlap (saved rule)" : "Minimum required experience"}</dt><dd>{rules?.experience_min == null && rules?.experience_max == null ? "Any" : `${rules?.experience_min ?? 0}–${rules?.experience_max ?? "unbounded"} years, inclusive`}{rules?.experience_include_unknown ? " · Include unknown" : " · Exclude unknown when constrained"}</dd></div>
              <div><dt className="font-semibold">New Grad</dt><dd>{rules?.new_grad || "any"} (independent of years of experience)</dd></div>
            </dl>
          </section>
          <details><summary className="cursor-pointer min-h-9">All configured rules</summary><pre className="font-code whitespace-pre-wrap break-words text-xs">{workflowSpecToYaml(draft)}</pre></details>
        </section>}
        {step === "Review" && <WorkflowPreviewResults preview={preview} draft={draft} setError={setError} />}
        {advanced && <section className="grid gap-3 border-t pt-3">
          <div className="flex flex-wrap gap-2"><Button disabled={locked} onClick={() => yaml == null ? setYaml(workflowSpecToYaml(model.commitInputs())) : void model.applyYaml()}>{yaml == null ? "Edit YAML" : "Apply YAML to form"}</Button><Button disabled={yaml != null} onClick={() => { const url = URL.createObjectURL(new Blob([workflowSpecToYaml(model.commitInputs())], { type: "text/yaml" })); const link = document.createElement("a"); link.href = url; link.download = "workflow.yaml"; link.click(); URL.revokeObjectURL(url); }}>Export YAML</Button></div>
          <label className="grid gap-2">Load example workflow
            <Select aria-label="Load example workflow" value="" disabled={locked} options={[{ value: "", label: "Choose an example…" }, ...SAMPLE_WORKFLOW_TEMPLATES.map(template => ({value: template.id, label: template.name}))]}
              onChange={id => { const template = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === id); if (template) model.replaceImport(structuredClone(template.spec)); }} />
          </label>
        </section>}
      </>}
    </div>
  </Modal>;
}
