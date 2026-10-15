import { useEffect, useRef, useState } from "react";
import { parseDocument } from "yaml";
import type { Source } from "../bindings/Source";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import { api } from "../lib/api";
import { sourceToWorkflowSpec, canonicalizeAtsUrl } from "../lib/yamlWorkflow";
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
export function useWorkflowDraft({ isOpen, initialSource, onSourceSaved }: {
 isOpen: boolean; initialSource?: Source | null; onSourceSaved: () => void;
}) {
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
    if (next.extractor.url !== draft.extractor.url) setDetected(advanced ? next.extractor.url : "");
  };
  const detect = async () => {
    const current = ++revision.current;
    setBusy(true);
    setError("");
    setPreview(null);
    try {
      const result = await api.detectSource(draft.extractor.url);
      if (revision.current !== current) return;
      const detectedDraft: WorkflowSpec = {
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
      };
      setDraft(detectedDraft);
      setDetected(canonicalizeAtsUrl(result.kind, result.identifier));
      return detectedDraft;
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
    let effectiveDraft = draft;
    if (!advanced && !detected) {
      const detectedDraft = await detect();
      if (!detectedDraft) return;
      effectiveDraft = detectedDraft;
    }
    const current = ++revision.current;
    const id = crypto.randomUUID();
    request.current = id;
    setBusy(true);
    setError("");
    setPreview(null);
    try {
      const result = await api.previewWorkflow(effectiveDraft, id);
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
  return { draft, change, advanced, setAdvanced, yaml, setYaml, detected, preview, setPreview, error, setError, busy, setBusy, saved, fileInput, content, request, cancel, detect, applyYaml, runPreview, save };
}
