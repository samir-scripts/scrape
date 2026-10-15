import { useEffect, useRef, useState } from "react";
import type { Source } from "../bindings/Source";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import { api } from "../lib/api";
import { importWorkflowYaml } from "../lib/workflowImport";
import { sourceToWorkflowSpec, canonicalizeAtsUrl } from "../lib/yamlWorkflow";

export const WORKFLOW_STEPS = ["Source", "Matching", "Schedule", "Review"] as const;
export type WorkflowStep = typeof WORKFLOW_STEPS[number];
const newDraft = (): WorkflowSpec => ({
  version: 3, metadata: { name: "", enabled: true }, extractor: { kind: "jsonld", url: "" },
  schedule: { cadence: "daily", time: "09:00", timezone: Intl.DateTimeFormat().resolvedOptions().timeZone },
  filters: {}, retention: { days: 30, policy: "untracked_only" },
});
export function useWorkflowDraft({ isOpen, initialSource, initialSpec, onSourceSaved }: {
  isOpen: boolean; initialSource?: Source | null; initialSpec?: WorkflowSpec | null; onSourceSaved: () => void;
}) {
  const [draft, setDraft] = useState<WorkflowSpec>(newDraft);
  const [step, setStep] = useState<WorkflowStep>("Source"), [visited, setVisited] = useState(0);
  const [advanced, setAdvanced] = useState(false), [yaml, setYamlState] = useState<string | null>(null);
  const [detected, setDetected] = useState("");
  const [preview, setPreview] = useState<WorkflowPreview | null>(null), [previewRevision, setPreviewRevision] = useState<number | null>(null);
  const [error, setError] = useState(""), [errorStep, setErrorStep] = useState<WorkflowStep>("Source");
  const [operation, setOperation] = useState<"detect" | "preview" | "validate" | "save" | "run" | null>(null);
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
      invalidate(); setDraft(initialSpec || (initialSource ? sourceToWorkflowSpec(initialSource) : newDraft()));
      setDetected(initialSpec?.extractor.url || initialSource?.identifier || "");
      setStep("Source"); setVisited(0); setAdvanced(false); setYamlState(null); setError(""); setSaved(null); setDirty(false);
    }
    return () => {
      if (request.current) void api.cancelWorkflowPreview(request.current).catch(() => {});
      request.current = null; revision.current++;
    };
  }, [isOpen, initialSource, initialSpec]);
  const change = (next: WorkflowSpec) => {
    invalidate(); setDirty(true); setDraft(next); setError("");
    if (next.extractor.url !== draftRef.current.extractor.url) setDetected(advanced ? next.extractor.url : "");
  };
  const setYaml = (value: string | null) => { invalidate(); setDirty(true); setYamlState(value); };
  const report = (value: unknown, fallback: WorkflowStep = step) => {
    const message = String(value); setError(message);
    setErrorStep(/filters|query|matching|compensation/i.test(message) ? "Matching" : /schedule|timezone|retention/i.test(message) ? "Schedule" : /extractor|metadata|source|url/i.test(message) ? "Source" : fallback);
  };
  const detect = async () => {
    const current = ++revision.current; setOperation("detect"); setError(""); setPreview(null);
    const input = draftRef.current;
    try {
      const result = await api.detectSource(input.extractor.url);
      if (revision.current !== current) return;
      const url = canonicalizeAtsUrl(result.kind, result.identifier);
      const next: WorkflowSpec = { ...input, metadata: { ...input.metadata, name: input.metadata.name || result.name, company: input.metadata.company || result.name }, extractor: { ...input.extractor, kind: result.kind as WorkflowSpec["extractor"]["kind"], url } };
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
  const replaceImport = (spec: WorkflowSpec) => {
    if (dirty && !window.confirm("Replace unsaved workflow changes with this validated YAML draft?")) return;
    change(spec); setDetected(spec.extractor.url); setYamlState(null); setStep("Source"); setVisited(0);
  };
  const applyYaml = async () => {
    const current = ++revision.current; setOperation("validate");
    try {
      const validated = await importWorkflowYaml(yaml || "");
      if (revision.current !== current) return;
      change(validated); setDetected(validated.extractor.url); setYamlState(null);
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) setOperation(null); }
  };
  const navigate = (target: WorkflowStep) => { setStep(target); setError(""); };
  const next = async () => {
    if (yaml != null) { report("Apply YAML to the form before continuing."); return; }
    if (invalidInputs()) return;
    if (step === "Source") {
      if (!draft.extractor.url.trim()) { report("Source URL is required.", "Source"); return; }
      const input = !advanced && !detected ? await detect() : draft;
      if (!input) return;
      if (!input.metadata.name.trim()) { report("Workflow name is required.", "Source"); return; }
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
    let effective = draft;
    if (!advanced && !detected) { const result = await detect(); if (!result) return; effective = result; }
    const current = ++revision.current, id = crypto.randomUUID();
    request.current = id; setOperation("preview"); setError(""); setPreview(null);
    try {
      const result = await api.previewWorkflow(effective, id);
      if (revision.current === current) { setPreview(result); setDraft(result.spec); setPreviewRevision(current); }
    } catch (error) { if (revision.current === current) report(error); }
    finally { if (revision.current === current) { request.current = null; setOperation(null); } }
  };
  const canSave = !!preview && previewRevision === revision.current && !preview.errors.length && yaml == null && !operation;
  const save = async () => {
    if (!canSave) return;
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
    try { const run = await api.runSourceNow(saved.id); if (["failed", "incomplete"].includes(run.status)) throw new Error(run.error || "Workflow run failed."); return true; }
    catch (error) { report(error); return false; }
    finally { setOperation(null); }
  };
  return { draft, change, step, visited, navigate, next, advanced, setAdvanced, yaml, setYaml, detected, preview, error, errorStep, setError, operation, busy: !!operation, saved, content, cancel, detect, applyYaml, runPreview, save, canSave, replaceImport, runSaved };
}
