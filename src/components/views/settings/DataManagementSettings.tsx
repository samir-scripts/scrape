import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { CleanupMode } from "../../../bindings/CleanupMode";
import type { CleanupPreview } from "../../../bindings/CleanupPreview";
import type { CleanupResult } from "../../../bindings/CleanupResult";
import { api } from "../../../lib/api";
import { errorMessage } from "../../../lib/errors";
import { applyDataCleanup } from "../../../lib/dataCleanup";
import { Modal } from "../../ui/Modal";
import { Button } from "../../ui/Button";
import { formatDateTime } from "../../../lib/formatters";

const OPERATIONS: { mode: CleanupMode; title: string; description: string }[] = [
  { mode: "all_jobs", title: "Delete all jobs", description: "Clear active and removed jobs, including favorites, notes and application history." },
  { mode: "except_favorites", title: "Delete except favorites", description: "Keep every saved copy of a posting if any copy is favorited, with its notes and history." },
  { mode: "except_tracked", title: "Delete except tracked applications", description: "Keep every copy of Applied, Interviewing, Offer and Rejected postings, with their saved state." },
  { mode: "reset_all", title: "Reset all application data", description: "Clear jobs, workflows, settings, run logs, notifications, blocks and interface preferences. Leave automatic scraping paused." },
];

export function DataManagementSettings() {
  const client = useQueryClient();
  const [mode, setMode] = useState<CleanupMode | null>(null);
  const [preview, setPreview] = useState<CleanupPreview | null>(null);
  const [pause, setPause] = useState(true), [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false), [loading, setLoading] = useState(false);
  const [error, setError] = useState(""), [result, setResult] = useState<CleanupResult | null>(null);
  const cancel = useRef<HTMLButtonElement>(null), generation = useRef(0);
  const operation = OPERATIONS.find(item => item.mode === mode);
  const desktop = isTauri();
  const retentionStatus = useQuery({
    queryKey: ["retention_status"],
    queryFn: api.getRetentionStatus,
    enabled: desktop,
    retry: false,
  }).data;
  useEffect(() => { if (mode) cancel.current?.focus(); }, [mode]);
  useEffect(() => () => { generation.current++; }, []);
  const close = () => { if (!busy) { generation.current++; setMode(null); } };
  async function load(selected: CleanupMode) {
    const request = ++generation.current;
    setLoading(true); setError(""); setPreview(null);
    try { const data = await api.previewDataCleanup(selected); if (request === generation.current) setPreview(data); }
    catch (error) { if (request === generation.current) setError(errorMessage(error)); }
    finally { if (request === generation.current) setLoading(false); }
  }
  async function execute() {
    if (!preview || busy) return;
    setBusy(true); setError("");
    try {
      const data = await api.executeDataCleanup({ preview, pause_after: pause, confirmation });
      // Clear the dialog before invalidating app data. No optimistic deletion.
      setMode(null); setResult(data);
      await applyDataCleanup(client, data);
    } catch (error) { setError(errorMessage(error)); setPreview(null); }
    finally { setBusy(false); }
  }
  return <section className="max-w-3xl space-y-5">
    <div><h1 className="text-xl font-bold">Data management</h1>
      <p className="mt-2 text-sm">Permanently clear saved data. Each cleanup creates a consistent database recovery backup first.</p></div>
    {desktop && retentionStatus?.last_attempt_at && <div className="border p-3 text-sm space-y-1" aria-label="Automatic retention status">
      <p>Last automatic cleanup: {formatDateTime(retentionStatus.last_attempt_at)} · {retentionStatus.last_deleted_records} records removed.</p>
      {retentionStatus.last_error && <p className="text-[var(--color-danger)]">Cleanup error: {retentionStatus.last_error}</p>}
    </div>}
    {desktop && retentionStatus && !retentionStatus.last_attempt_at && <p role="status" className="border p-3 text-sm">Automatic retention has not completed its first startup sweep yet.</p>}
    {desktop && retentionStatus && retentionStatus.consecutive_failures >= 3 && <div role="alert" className="border border-[var(--color-danger)] p-3 text-sm text-[var(--color-danger)]">
      <p className="font-bold">Automatic retention has failed {retentionStatus.consecutive_failures} consecutive times.</p>
      <p>Check that the database is writable and enough disk space is available. Cleanup retries automatically after the next 24-hour interval.</p>
    </div>}
    {!desktop && <p role="status" className="border p-3 text-sm">Available in the desktop app only. Browser previews cannot clear the desktop database.</p>}
    {result && <div role="status" className="border p-4 text-sm space-y-2 select-text">
      <p>{result.counts.deleted_records} saved records deleted; {result.counts.preserved_records} preserved. Automatic scraping {result.scheduling_paused ? "is paused" : "keeps its existing setting"}.</p>
      {result.warnings.map(warning => <p key={warning} role="alert">{warning}</p>)}
      <p>Recovery backup: <span className="break-all">{result.backup_path}</span></p>
      <p>This action has no Undo. Restoring a backup replaces current data.</p>
      <details><summary data-control-action="settings.data.recovery_instructions" className="cursor-pointer underline">Recovery instructions</summary>
        <ol className="list-decimal pl-5 mt-2 space-y-2">
          <li>Quit Scrape completely, including its tray or menu-bar process.</li>
          <li>Find scrape_data.db beside the backups folder. Move the current database and its -wal and -shm files together to a safe location.</li>
          <li>Copy the recovery backup above into that folder as scrape_data.db. Never combine it with the old WAL or SHM files.</li>
          <li>Start the compatible app version and review automatic scheduling before resuming. The backup contains the settings from before cleanup.</li>
        </ol>
      </details>
    </div>}
    <div className="divide-y border-y">
      {OPERATIONS.map(item => <div key={item.mode} className="py-4 flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1 basis-80"><h2 className="font-bold">{item.title}</h2><p className="text-sm mt-1">{item.description}</p></div>
        <Button data-control-action="settings.data.review_cleanup" variant="danger" disabled={!desktop || busy} onClick={() => {
          setMode(item.mode); setConfirmation(""); setPause(true); void load(item.mode);
        }}>Review {item.mode === "reset_all" ? "reset" : item.mode === "all_jobs" ? "all jobs" : item.mode === "except_favorites" ? "keep favorites" : "keep tracked"}</Button>
      </div>)}
    </div>
    <Modal isOpen={mode !== null} onClose={close} title={operation?.title || "Confirm cleanup"} footer={<div className="flex flex-wrap justify-end gap-3">
      <button data-control-action="settings.data.cancel_cleanup" ref={cancel} type="button" className="border px-3 py-2" disabled={busy} onClick={close}>Cancel</button>
      <Button data-control-action="settings.data.confirm_cleanup" variant="danger" disabled={busy || loading || !preview || (mode === "reset_all" && confirmation !== "RESET")} onClick={() => void execute()}>{busy ? "Cleaning up…" : "Permanently delete"}</Button>
    </div>}>
      <div className="space-y-4 text-sm">
        <p>{operation?.description}</p>
        {loading && <p role="status">Calculating affected data…</p>}
        {preview && <div className="border p-3 space-y-2" aria-label="Cleanup counts">
          <p>Delete {preview.deleted_postings} postings ({preview.deleted_records} saved records).</p>
          <p>Preserve {preview.preserved_postings} postings ({preview.preserved_records} saved records).</p>
          <p>Delete {preview.deleted_notes} saved notes and {preview.deleted_history} history events. Soft-removed jobs are included.</p>
          <p>{mode === "reset_all" ? "Delete" : "Keep"} {preview.workflows} workflows and {preview.blocks} suppression entries{mode !== "reset_all" && ", plus settings, run logs and notifications"}.</p>
        </div>}
        <p className="text-[var(--color-danger)]">Deletion is permanent. A recovery backup is required before deleting. Active scrapes will be stopped. Jobs may return in future scrapes unless already blocked.</p>
        {mode === "reset_all" ? <><p>All blocks are cleared. Automatic scraping stays paused.</p><label className="flex flex-col gap-1">Type RESET to confirm<input data-control-action="settings.data.reset_confirmation" autoComplete="off" value={confirmation} onChange={event => setConfirmation(event.target.value)} disabled={busy} className="border bg-[var(--color-paper)] p-2" /></label></> :
          <label className="flex items-start gap-2"><input data-control-action="settings.data.pause_after_cleanup" type="checkbox" checked={pause} onChange={event => setPause(event.target.checked)} disabled={busy} />Pause automatic scraping after cleanup</label>}
        {error && <div role="alert"><p>{error}</p><button data-control-action="settings.data.refresh_counts" className="border px-3 py-2 mt-2" disabled={busy || loading} onClick={() => mode && void load(mode)}>Refresh cleanup counts</button></div>}
      </div>
    </Modal>
  </section>;
}
