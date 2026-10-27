import { useEffect, useRef, useState } from "react";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import { importWorkflowYaml } from "../../../lib/workflowImport";
import { IconButton } from "../../ui/IconButton";
import { Icon } from "../../ui/Icon";

export function WorkflowImportButton({ onImport, disabled = false }: {
  onImport: (spec: WorkflowSpec) => void; disabled?: boolean;
}) {
  const imported = useRef(onImport); imported.current = onImport;
  const input = useRef<HTMLInputElement>(null), revision = useRef(0);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  useEffect(() => () => { revision.current++; }, []);
  useEffect(() => { if (disabled) { revision.current++; setBusy(false); } }, [disabled]);
  return <div className="grid gap-1">
    <IconButton data-control-action="workflow.import_yaml" label="Import workflow YAML" disabled={disabled || busy} onClick={() => input.current?.click()}><Icon name="upload" /></IconButton>
    <input data-control-action="workflow.import_file" ref={input} type="file" accept=".yaml,.yml" aria-label="Workflow YAML file" className="hidden" disabled={disabled || busy}
      onChange={async event => {
        const file = event.target.files?.[0]; event.target.value = "";
        if (!file) return;
        const current = ++revision.current; setBusy(true); setError("");
        try {
          const spec = await importWorkflowYaml(await file.text());
          if (revision.current === current) imported.current(spec);
        } catch (error) { if (revision.current === current) setError(String(error)); }
        finally { if (revision.current === current) setBusy(false); }
      }} />
    {busy && <span role="status" className="text-xs">Validating YAML…</span>}
    {error && <p role="alert" className="text-sm text-[var(--color-danger)]">{error}</p>}
  </div>;
}
