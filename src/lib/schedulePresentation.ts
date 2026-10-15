import type { SchedulerStatus } from "../bindings/SchedulerStatus";

/** Formats backend instants only. Include the date, even across day/year boundaries. */
export function scheduleTime(value: string | null, timezone: string): string | null {
  if (!value || !Number.isFinite(Date.parse(value))) return null;
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: "short", day: "numeric", year: "numeric", hour: "2-digit", minute: "2-digit", hour12: false, timeZone: timezone,
    }).format(new Date(value));
  } catch { return null; }
}

export function schedulePresentation(status?: SchedulerStatus, error?: unknown, loading = false, scanning = false, desktop = true) {
  if (!desktop) return { label: "Desktop required", detail: "Browser examples do not run an automatic scheduler." };
  const running = scanning || status?.running;
  const active = running ? `Running${status && !status.enabled ? " · Paused" : ""}` : "";
  if (error) return { label: active ? `${active} · Schedule unavailable` : "Schedule unavailable", detail: `Could not read scheduling status: ${String(error)}. Any previously displayed schedule is stale.` };
  if (!status || loading) return { label: active || "Loading schedule", detail: "Reading scheduler status…" };
  let label: string, detail: string;
  if (!status.enabled) { label = "Paused"; detail = "Automatic scheduling is paused. Manual runs remain available. An active run continues until completed or cancelled."; }
  else if (status.enabled_workflows === 0) { label = "No enabled workflows"; detail = "Create or enable a workflow in Scraper."; }
  else if (status.schedules.length === 0) { label = "Manual only"; detail = "Enabled workflows have no automatic schedule. Use Run all or an individual Run action in Scraper."; }
  else {
    const next = status.schedules.find(s => s.next_run_at === status.next_run_at);
    const formatted = next && scheduleTime(next.next_run_at, next.timezone);
    if (!formatted || status.schedules.some(s => !scheduleTime(s.next_run_at, s.timezone))) {
      label = "Schedule needs attention"; detail = "An automatic workflow has no valid next run or timezone. Refresh status or edit its schedule in Scraper.";
    } else { label = `Next ${formatted}`; detail = `Next: ${status.schedules.filter(s => s.next_run_at === status.next_run_at).map(s => s.name).join(", ")} · ${next!.timezone}. Due work waits for any active run to finish.`; }
  }
  return { label: active ? `${active}${label === "Paused" ? "" : ` · ${label}`}` : label, detail };
}
