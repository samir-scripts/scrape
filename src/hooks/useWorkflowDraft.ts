import { COMMIT_KEYWORDS } from "../components/ui/KeywordInput";
import { INGESTION_LIMITS } from "../bindings/ingestionLimits";
import { useEffect, useRef, useState } from "react";
import type { Source } from "../bindings/Source";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import type { WorkflowSourceSpec } from "../bindings/WorkflowSourceSpec";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import { api } from "../lib/api";
import { importWorkflowYaml } from "../lib/workflowImport";
import { sourceToWorkflowSpec, canonicalizeAtsUrl, normalizedEndpoint } from "../lib/yamlWorkflow";
import { companyOptions, selectedCompanyId } from "../components/views/settings/CompanyPicker";
import { emptyRules } from "../components/views/settings/WorkflowSections";

export const WORKFLOW_STEPS = ["Roles", "Criteria", "Sources", "Schedule"] as const;
export type WorkflowStep = typeof WORKFLOW_STEPS[number];
export type WorkflowDraftChange = (next: WorkflowSpec | ((current: WorkflowSpec) => WorkflowSpec)) => void;
const newDraft = (): WorkflowSpec => ({
  version: 8, max_jobs_per_run: INGESTION_LIMITS.default, metadata: { name: "", enabled: true },
  sources: [{ id: "source-1", label: "Cross-Company Search", enabled: true, extractor: { kind: "search", url: "" } }],
  schedule: { cadence: "daily", time: "09:00", timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, run_on_startup: false },
  filters: { matching: { ...emptyRules(), posting_date: { preset: "past_week", last_hours: null, from: null, through: null, timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, include_unknown: false } } }, retention: { days: 30, policy: "untracked_only" },
});
export function useWorkflowDraft({ isOpen, initialSource, initialSpec, onSourceSaved }: {
  isOpen: boolean; initialSource?: Source | null; initialSpec?: WorkflowSpec | null; onSourceSaved: () => void;
}) {
  const [draft, setDraft] = useState<WorkflowSpec>(newDraft);
  const [inputSession, setInputSession] = useState(0);
  const [step, setStep] = useState<WorkflowStep>("Roles"), [visited, setVisited] = useState(0);
  const [advanced, setAdvanced] = useState(false), [yaml, setYamlState] = useState<string | null>(null);
  const [detected, setDetected] = useState("");
  const [preview, setPreview] = useState<WorkflowPreview | null>(null), [previewRevision, setPreviewRevision] = useState<number | null>(null);
  const [error, setError] = useState(""), [errorStep, setErrorStep] = useState<WorkflowStep>("Roles");
  const [operation, setOperation] = useState<"detect" | "preview" | "validate" | "save" | "run" | null>(null);
  const [pendingImport, setPendingImport] = useState<WorkflowSpec | null>(null);
  const [pendingCompany, setPendingCompany] = useState<{ sourceId: string; companyId: string } | null>(null);
  const [saved, setSaved] = useState<Source | null>(null), [dirty, setDirty] = useState(false);
  const imported = useRef(false);
  const userNamed = useRef(false);
  const content = useRef<HTMLDivElement>(null), revision = useRef(0), request = useRef<string | null>(null);
  const draftRef = useRef(draft); draftRef.current = draft;
  const cancel = () => {
    if (request.current) void api.cancelWorkflowPreview(request.current).catch(() => {});
    request.current = null; revision.current++; setOperation(null); setPreview(null); setPreviewRevision(null);
  };
  const invalidate = () => { cancel(); setPreview(null); setPreviewRevision(null); };
  useEffect(() => {
    if (isOpen) {
      imported.current = !!initialSpec;
      setInputSession(value => value + 1);
      invalidate();
      const initial = initialSpec || (initialSource ? sourceToWorkflowSpec(initialSource) : newDraft());
      userNamed.current = !!initialSpec || !!initialSource;
      draftRef.current = initial; setDraft(initial);
      setDetected(initial.sources?.[0]?.extractor.url || initialSource?.identifier || "");
      setStep("Roles"); setVisited(0); setAdvanced(false); setYamlState(null); setError(""); setSaved(null); setPendingImport(null); setPendingCompany(null); setDirty(false);
    }
    return () => {
      if (request.current) void api.cancelWorkflowPreview(request.current).catch(() => {});
      request.current = null; revision.current++;
    };
  }, [isOpen, initialSource, initialSpec]);
  const change: WorkflowDraftChange = (update) => {
    const previous = draftRef.current;
    let next = typeof update === "function" ? update(previous) : update;
    const nameChanged = next.metadata.name !== previous.metadata.name;
    if (nameChanged) userNamed.current = !!next.metadata.name.trim();
    if (!userNamed.current && !nameChanged) {
      const roles = next.filters?.matching?.target_roles || [], companies = next.filters?.matching?.companies || [];
      const name = roles.length ? `${roles.join(" & ")}${companies.length ? ` at ${companies.join(" & ")}` : " jobs"}` : "New Job Workflow";
      next = { ...next, metadata: { ...next.metadata, name } };
    }
    invalidate(); setDirty(true); draftRef.current = next; setDraft(next); setError("");
    const prevUrl = previous.sources?.[0]?.extractor?.url;
    const nextUrl = next.sources?.[0]?.extractor?.url;
    if (nextUrl !== prevUrl) setDetected(advanced ? (nextUrl || "") : "");
  };
  const applyCompany = (sourceId: string, companyId: string) => {
    const option = companyOptions.find(option => option.company.id === companyId);
    const current = draftRef.current;
    const target = current.sources.find(source => source.id === sourceId);
    if (!option || !target) return;
    change({
      ...current,
      sources: current.sources.map(source => source.id === sourceId ? {
        ...source, label: option.source.label, employer_hint: option.source.employer_hint,
        enabled: source.enabled && option.source.enabled && option.template.readiness === "ready" && !option.source.extractor.script && option.source.extractor.mode !== "script",
        extractor: structuredClone(option.source.extractor),
      } : source),
    });
    setPendingCompany(null);
    if (current.sources[0]?.id === sourceId) setDetected(option.source.extractor.url);
  };
  const selectCompany = (sourceId: string, companyId: string): void => {
    const source = draftRef.current.sources.find(source => source.id === sourceId);
    if (!source || selectedCompanyId(source) === companyId) return;
    if (source.extractor.url.trim() || source.extractor.script) setPendingCompany({ sourceId, companyId });
    else applyCompany(sourceId, companyId);
  };
  const commitInputs = () => {
    content.current?.querySelectorAll("input[data-keyword-input]").forEach(input => input.dispatchEvent(new Event(COMMIT_KEYWORDS)));
    return draftRef.current;
  };
  const setYaml = (value: string | null) => { invalidate(); setDirty(true); setYamlState(value); };
  const report = (value: unknown, fallback: WorkflowStep = step) => {
    const message = String(value); setError(message);
    setErrorStep(/target_roles|keywords|companies|metadata/i.test(message) ? "Roles" : /filters|query|matching|compensation|max_jobs_per_run|role_types|date|posting_date|location|experience|workplace/i.test(message) ? "Criteria" : /schedule|timezone|retention/i.test(message) ? "Schedule" : /extractor|source|url/i.test(message) ? "Sources" : fallback);
  };
  const detect = async (sourceIndex: number = 0) => {
    invalidate();
    const current = ++revision.current; setOperation("detect"); setError(""); setPreview(null);
    const input = draftRef.current;
    const targetSource = input.sources?.[sourceIndex] || input.sources?.[0];
    if (!targetSource || !targetSource.extractor.url.trim()) {
      report("Source URL is required.", "Sources");
      setOperation(null);
      return;
    }
    try {
      const result = await api.detectSource(targetSource.extractor.url);
      if (revision.current !== current) return;
      const url = canonicalizeAtsUrl(result.kind, result.identifier);
      const isDefaultLabel = !targetSource.label || targetSource.label === "Career Board" || /^Career Board \d+$/.test(targetSource.label);
      const updatedSource: WorkflowSourceSpec = {
        ...targetSource,
        label: isDefaultLabel && result.name ? result.name : (targetSource.label || result.name),
        employer_hint: targetSource.employer_hint || result.name || undefined,
        extractor: {
          ...targetSource.extractor,
          kind: result.kind as WorkflowSourceSpec["extractor"]["kind"],
          url,
        },
      };
      const nextSources = [...input.sources];
      nextSources[sourceIndex] = updatedSource;
      const next: WorkflowSpec = {
        ...input,
        metadata: {
          ...input.metadata,
          company: input.metadata.company || result.name,
        },
        sources: nextSources,
      };
      change(next); setDetected(url); return next;
    } catch (error) { if (revision.current === current) report(`${String(error)} Open Advanced settings to configure an adapter or selectors.`, "Sources"); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const invalidInputs = () => {
    const invalid = content.current?.querySelector<HTMLElement>("[aria-invalid=true]");
    if (!invalid) return false;
    setErrorStep((invalid.closest("[data-workflow-step]") as HTMLElement)?.dataset.workflowStep as WorkflowStep || step);
    setError("Fix the highlighted inputs before continuing."); return true;
  };
  const applyImport = (spec: WorkflowSpec) => {
    imported.current = true; userNamed.current = true;
    setInputSession(value => value + 1);
    change(spec); setDetected(spec.sources?.[0]?.extractor.url || ""); setYamlState(null); setStep("Roles"); setVisited(0); setPendingImport(null);
  };
  const replaceImport = (spec: WorkflowSpec) => { if (dirty) setPendingImport(spec); else applyImport(spec); };
  const confirmImport = () => { if (pendingImport) applyImport(pendingImport); };
  const cancelImport = () => setPendingImport(null);
  const applyYaml = async () => {
    const current = ++revision.current; setOperation("validate");
    try {
      const validated = await importWorkflowYaml(yaml || "");
      if (revision.current !== current) return;
      setInputSession(value => value + 1);
      imported.current = true; userNamed.current = true;
      change(validated); setDetected(validated.sources?.[0]?.extractor.url || ""); setYamlState(null);
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const navigate = (target: WorkflowStep) => { commitInputs(); setStep(target); setError(""); };
  const next = async () => {
    const draft = commitInputs();
    if (yaml != null) { report("Apply YAML to the form before continuing."); return; }
    if (invalidInputs()) return;
    if (step === "Roles" && !draft.metadata.name.trim()) {
      const roles = draft.filters?.matching?.target_roles || [];
      const companies = draft.filters?.matching?.companies || [];
      const name = roles.length ? `${roles.join(" & ")}${companies.length ? ` at ${companies.join(" & ")}` : " jobs"}` : "New Job Workflow";
      const named = { ...draft, metadata: { ...draft.metadata, name } };
      draftRef.current = named; setDraft(named);
    }
    if (step === "Sources") {
      if (!draft.sources || draft.sources.length === 0) { report("At least one source is required.", "Sources"); return; }
      if (draft.metadata.enabled !== false && !draft.sources.some(s => s.enabled)) { report("At least one source must be enabled.", "Sources"); return; }
      for (let i = 0; i < draft.sources.length; i++) {
        const s = draft.sources[i];
        if (!s.label.trim()) { report(`Source ${i + 1} label is required.`, "Sources"); return; }
        if (s.extractor.kind !== "search" && !s.extractor.url.trim()) { report(`Source '${s.label}' URL is required.`, "Sources"); return; }
      }
      const seen = new Set<string>();
      for (const s of draft.sources) {
        const ep = normalizedEndpoint(s.extractor.kind, s.extractor.url);
        if (seen.has(ep)) {
          report(`Duplicate source endpoint: '${s.extractor.url}' conflicts with another source in this workflow.`, "Sources");
          return;
        }
        seen.add(ep);
      }
      if (!advanced && !detected && draft.sources[0]?.extractor.url.trim() && draft.sources[0].extractor.kind !== "search") {
        const input = await detect(0);
        if (!input) return;
      }
    }
    if (step === "Sources") {
      const current = revision.current; setOperation("validate");
      try { await api.validateWorkflow(draftRef.current); if (revision.current !== current) return; }
      catch (error) { if (revision.current === current) report(error); return; }
      finally { if (revision.current === current) setOperation(null); }
    }
    const index = Math.min(3, WORKFLOW_STEPS.indexOf(step) + 1); setVisited(value => Math.max(value, index)); navigate(WORKFLOW_STEPS[index]);
  };
  const runPreview = async () => {
    if (invalidInputs()) return;
    let effective = commitInputs();
    if (!advanced && !detected && effective.sources?.[0]?.extractor.kind !== "search" && effective.sources?.[0]?.extractor.url.trim()) {
      const result = await detect(0);
      if (!result) return;
      effective = result;
    }
    const current = ++revision.current, id = crypto.randomUUID();
    request.current = id; setOperation("preview"); setError(""); setPreview(null);
    try {
      const result = await api.previewWorkflow(effective, id);
      if (revision.current === current) { draftRef.current = result.spec; setPreview(result); setDraft(result.spec); setPreviewRevision(current); }
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) { request.current = null; setOperation(null); } }
  };
  const canSave = !!preview && previewRevision === revision.current && !preview.errors.length && yaml == null && !operation && !pendingImport && !pendingCompany;
  const save = async () => {
    const draft = commitInputs();
    if (!canSave || previewRevision !== revision.current || invalidInputs()) return;
    const current = revision.current; setOperation("save"); setError("");
    try {
      const source = await api.saveWorkflow(draft, initialSource?.id, imported.current); onSourceSaved();
      if (revision.current === current) { setSaved(source); setDirty(false); }
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const saveDisabled = async () => {
    if (operation || pendingImport || pendingCompany || yaml != null || invalidInputs()) return;
    const input = commitInputs();
    const current = revision.current; setOperation("save"); setError("");
    try {
      const disabled = await api.validateWorkflow({ ...input, metadata: { ...input.metadata, enabled: false },
        sources: input.sources.map(source => source.extractor.script || source.extractor.mode === "script" ? { ...source, enabled: false } : source) });
      if (revision.current !== current) return;
      const source = await api.saveWorkflow(disabled, initialSource?.id, imported.current);
      onSourceSaved();
      if (revision.current === current) { setSaved(source); setDirty(false); }
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const runSaved = async () => {
    if (!saved) return false;
    setOperation("run"); setError("");
    try { const run = await api.runSourceNow(saved.id); if (["failed", "partial", "cancelled"].includes(run.status)) throw new Error(run.error || "Workflow run failed."); return true; }
    catch (error) { report(error); return false; }
    finally { setOperation(null); }
  };
  return { draft, change, commitInputs, inputSession, step, visited, navigate, next, advanced, setAdvanced, yaml, setYaml, detected, preview, error, errorStep, setError, operation, busy: !!operation, saved, content, cancel, invalidate, detect, applyYaml, runPreview, save, saveDisabled, canSave, replaceImport, pendingImport, confirmImport, cancelImport, runSaved, selectCompany, pendingCompany, confirmCompany: () => { if (pendingCompany) applyCompany(pendingCompany.sourceId, pendingCompany.companyId); }, cancelCompany: () => setPendingCompany(null) };
}
