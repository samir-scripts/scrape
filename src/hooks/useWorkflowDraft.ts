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

export const WORKFLOW_STEPS = ["Source", "Matching", "Schedule", "Review"] as const;
export type WorkflowStep = typeof WORKFLOW_STEPS[number];
export type WorkflowDraftChange = (next: WorkflowSpec | ((current: WorkflowSpec) => WorkflowSpec)) => void;
const newDraft = (): WorkflowSpec => ({
  version: 5, max_jobs_per_run: INGESTION_LIMITS.default, metadata: { name: "", enabled: true },
  sources: [{ id: "source-1", label: "Career Board 1", enabled: true, extractor: { kind: "jsonld", url: "" } }],
  schedule: { cadence: "daily", time: "09:00", timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, run_on_startup: false },
  filters: {}, retention: { days: 30, policy: "untracked_only" },
});
export function useWorkflowDraft({ isOpen, initialSource, initialSpec, onSourceSaved }: {
  isOpen: boolean; initialSource?: Source | null; initialSpec?: WorkflowSpec | null; onSourceSaved: () => void;
}) {
  const [draft, setDraft] = useState<WorkflowSpec>(newDraft);
  const [inputSession, setInputSession] = useState(0);
  const [step, setStep] = useState<WorkflowStep>("Source"), [visited, setVisited] = useState(0);
  const [advanced, setAdvanced] = useState(false), [yaml, setYamlState] = useState<string | null>(null);
  const [detected, setDetected] = useState("");
  const [preview, setPreview] = useState<WorkflowPreview | null>(null), [previewRevision, setPreviewRevision] = useState<number | null>(null);
  const [error, setError] = useState(""), [errorStep, setErrorStep] = useState<WorkflowStep>("Source");
  const [operation, setOperation] = useState<"detect" | "preview" | "validate" | "save" | "run" | null>(null);
  const [pendingImport, setPendingImport] = useState<WorkflowSpec | null>(null);
  const [saved, setSaved] = useState<Source | null>(null), [dirty, setDirty] = useState(false);
  const content = useRef<HTMLDivElement>(null), revision = useRef(0), request = useRef<string | null>(null);
  const draftRef = useRef(draft); draftRef.current = draft;
  const cancel = () => {
    if (request.current) void api.cancelWorkflowPreview(request.current).catch(() => {});
    request.current = null; revision.current++; setOperation(null);
  };
  const invalidate = () => { cancel(); setPreview(null); setPreviewRevision(null); };
  useEffect(() => {
    if (isOpen) {
      setInputSession(value => value + 1);
      invalidate();
      const initial = initialSpec || (initialSource ? sourceToWorkflowSpec(initialSource) : newDraft());
      setDraft(initial);
      setDetected(initial.sources?.[0]?.extractor.url || initialSource?.identifier || "");
      setStep("Source"); setVisited(0); setAdvanced(false); setYamlState(null); setError(""); setSaved(null); setPendingImport(null); setDirty(false);
    }
    return () => {
      if (request.current) void api.cancelWorkflowPreview(request.current).catch(() => {});
      request.current = null; revision.current++;
    };
  }, [isOpen, initialSource, initialSpec]);
  const change: WorkflowDraftChange = (update) => {
    const previous = draftRef.current;
    const next = typeof update === "function" ? update(previous) : update;
    invalidate(); setDirty(true); draftRef.current = next; setDraft(next); setError("");
    const prevUrl = previous.sources?.[0]?.extractor?.url;
    const nextUrl = next.sources?.[0]?.extractor?.url;
    if (nextUrl !== prevUrl) setDetected(advanced ? (nextUrl || "") : "");
  };
  const commitInputs = () => {
    content.current?.querySelectorAll("input[data-keyword-input]").forEach(input => input.dispatchEvent(new Event(COMMIT_KEYWORDS)));
    return draftRef.current;
  };
  const setYaml = (value: string | null) => { invalidate(); setDirty(true); setYamlState(value); };
  const report = (value: unknown, fallback: WorkflowStep = step) => {
    const message = String(value); setError(message);
    setErrorStep(/filters|query|matching|compensation|max_jobs_per_run/i.test(message) ? "Matching" : /schedule|timezone|retention/i.test(message) ? "Schedule" : /extractor|metadata|source|url/i.test(message) ? "Source" : fallback);
  };
  const detect = async (sourceIndex: number = 0) => {
    const current = ++revision.current; setOperation("detect"); setError(""); setPreview(null);
    const input = draftRef.current;
    const targetSource = input.sources?.[sourceIndex] || input.sources?.[0];
    if (!targetSource || !targetSource.extractor.url.trim()) {
      report("Source URL is required.", "Source");
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
          name: input.metadata.name || result.name,
          company: input.metadata.company || result.name,
        },
        sources: nextSources,
      };
      setDraft(next); draftRef.current = next; setDetected(url); return next;
    } catch (error) { if (revision.current === current) report(`${String(error)} Open Advanced settings to configure an adapter or selectors.`, "Source"); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const invalidInputs = () => {
    const invalid = content.current?.querySelector<HTMLElement>("[aria-invalid=true]");
    if (!invalid) return false;
    setErrorStep((invalid.closest("[data-workflow-step]") as HTMLElement)?.dataset.workflowStep as WorkflowStep || step);
    setError("Fix the highlighted inputs before continuing."); return true;
  };
  const applyImport = (spec: WorkflowSpec) => {
    setInputSession(value => value + 1);
    change(spec); setDetected(spec.sources?.[0]?.extractor.url || ""); setYamlState(null); setStep("Source"); setVisited(0); setPendingImport(null);
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
      change(validated); setDetected(validated.sources?.[0]?.extractor.url || ""); setYamlState(null);
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const navigate = (target: WorkflowStep) => { commitInputs(); setStep(target); setError(""); };
  const next = async () => {
    const draft = commitInputs();
    if (yaml != null) { report("Apply YAML to the form before continuing."); return; }
    if (invalidInputs()) return;
    if (step === "Source") {
      if (!draft.sources || draft.sources.length === 0) { report("At least one source is required.", "Source"); return; }
      if (!draft.sources.some(s => s.enabled)) { report("At least one source must be enabled.", "Source"); return; }
      for (let i = 0; i < draft.sources.length; i++) {
        const s = draft.sources[i];
        if (!s.label.trim()) { report(`Source ${i + 1} label is required.`, "Source"); return; }
        if (!s.extractor.url.trim()) { report(i === 0 ? "Source URL is required." : `Source '${s.label}' URL is required.`, "Source"); return; }
      }
      const seen = new Set<string>();
      for (const s of draft.sources) {
        const ep = normalizedEndpoint(s.extractor.kind, s.extractor.url);
        if (seen.has(ep)) {
          report(`Duplicate source endpoint: '${s.extractor.url}' conflicts with another source in this workflow.`, "Source");
          return;
        }
        seen.add(ep);
      }
      let currentDraft = draft;
      if (!advanced && !detected && draft.sources[0]?.extractor.url.trim()) {
        const input = await detect(0);
        if (!input) return;
        currentDraft = input;
      }
      if (!currentDraft.metadata.name.trim()) { report("Workflow name is required.", "Source"); return; }
    }
    if (step === "Schedule") {
      const current = revision.current; setOperation("validate");
      try { await api.validateWorkflow(draft); if (revision.current !== current) return; }
      catch (error) { if (revision.current === current) report(error); return; }
      finally { if (revision.current === current) setOperation(null); }
    }
    const index = Math.min(3, WORKFLOW_STEPS.indexOf(step) + 1); setVisited(value => Math.max(value, index)); navigate(WORKFLOW_STEPS[index]);
  };
  const runPreview = async () => {
    if (invalidInputs()) return;
    let effective = commitInputs();
    if (!advanced && !detected && effective.sources?.[0]?.extractor.url.trim()) {
      const result = await detect(0);
      if (!result) return;
      effective = result;
    }
    const current = ++revision.current, id = crypto.randomUUID();
    request.current = id; setOperation("preview"); setError(""); setPreview(null);
    try {
      const result = await api.previewWorkflow(effective, id);
      if (revision.current === current) { setPreview(result); setDraft(result.spec); setPreviewRevision(current); }
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) { request.current = null; setOperation(null); } }
  };
  const canSave = !!preview && previewRevision === revision.current && !preview.errors.length && yaml == null && !operation && !pendingImport;
  const save = async () => {
    const draft = commitInputs();
    if (!canSave || previewRevision !== revision.current) return;
    const current = revision.current; setOperation("save"); setError("");
    try {
      const source = await api.saveWorkflow(draft, initialSource?.id); onSourceSaved();
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
  return { draft, change, commitInputs, inputSession, step, visited, navigate, next, advanced, setAdvanced, yaml, setYaml, detected, preview, error, errorStep, setError, operation, busy: !!operation, saved, content, cancel, detect, applyYaml, runPreview, save, canSave, replaceImport, pendingImport, confirmImport, cancelImport, runSaved };
}
