import { useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { schedulePresentation, scheduleTime } from "../../lib/schedulePresentation";
import { useUIStore } from "../../lib/store";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";

export function ScheduleControl() {
  const [open, setOpen] = useState(false), [busy, setBusy] = useState(false), [actionError, setActionError] = useState("");
  const desktop = isTauri();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ["scheduler_status"], queryFn: api.getSchedulerStatus, enabled: desktop, retry: false });
  const { isScanning, scanStatusText, setView } = useUIStore();
  const status = query.data;
  const presentation = schedulePresentation(status, query.error, query.isPending, isScanning, desktop);
  const running = desktop && (isScanning || status?.running);
  async function act(action: () => Promise<unknown>) {
    setBusy(true); setActionError("");
    try { await action(); }
    catch (error) { setActionError(String(error)); }
    finally {
      await Promise.all(["scheduler_status", "settings", "scrape_runs"].map(key => client.invalidateQueries({ queryKey: [key] })));
      setBusy(false);
    }
  }
  return <>
    <button type="button" aria-label={`Schedule: ${presentation.label}`} aria-haspopup="dialog" aria-expanded={open}
      onClick={() => { setOpen(true); if (desktop) void query.refetch(); }}
      className="min-h-8 px-2 border border-transparent hover:border-[var(--color-ink)] text-xs text-[var(--color-ink)] max-w-72 text-left">
      <span className="inline-block w-2 h-2 mr-2 bg-current" aria-hidden="true" />{presentation.label}
    </button>
    <Modal isOpen={open} onClose={() => setOpen(false)} title="Schedule summary">
      <div className="space-y-4 text-sm">
        <p role="status">{presentation.detail}</p>
        {running && <p>{scanStatusText || "A worker run is active."}</p>}
        {desktop && status && !query.error && <ul className="divide-y divide-[var(--color-border)]">
          {status.schedules.map(s => <li key={s.id} className="py-3">
            <p className="font-bold">{s.name}</p>
            <p>{status.enabled ? "Next" : "Suspended"}: {scheduleTime(s.next_run_at, s.timezone) || "No valid next run"}</p>
            <p className="text-[var(--color-ink-mute)]">{s.timezone}</p>
          </li>)}
        </ul>}
        {actionError && <p role="alert" className="text-[var(--color-danger)]">{actionError}</p>}
        <div className="flex flex-wrap gap-2">
          {desktop && <Button disabled={busy || query.isFetching} onClick={() => void query.refetch()}>Refresh status</Button>}
          {desktop && status && !query.error && <Button disabled={busy} onClick={() => void act(() => api.setSchedulerPaused(status.enabled))}>{status.enabled ? "Pause automatic runs" : "Resume automatic runs"}</Button>}
          {running && <Button disabled={busy} onClick={() => void act(async () => {
            if (!await api.cancelScrape()) throw new Error("No active worker accepted cancellation. Refresh status.");
          })}>Cancel active run</Button>}
          <Button onClick={() => { setView("scraper"); setOpen(false); }}>Open Scraper</Button>
        </div>
      </div>
    </Modal>
  </>;
}
