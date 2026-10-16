import { clearCompanyLogoCache } from "./companyLogoCache";
import type { QueryClient } from "@tanstack/react-query";
import type { CleanupResult } from "../bindings/CleanupResult";
import type { RetentionStatus } from "../bindings/RetentionStatus";
import { DEFAULT_FILTER_SPEC, useUIStore } from "./store";

const lastApplied = new WeakMap<QueryClient, string>();

// Both IPC responses and application events use this one cache/state boundary.
export async function applyDataCleanup(client: QueryClient, result: CleanupResult) {
  if (lastApplied.get(client) === result.backup_path) return;
  lastApplied.set(client, result.backup_path);
  const reset = result.counts.mode === "reset_all";
  if (reset) {
    clearCompanyLogoCache();
    try {
      for (const key of Object.keys(localStorage)) if (key.startsWith("scrape:")) localStorage.removeItem(key);
    } catch { /* Restricted storage has no persistent preferences to clear. */ }
  }
  useUIStore.setState(state => ({
    selectedJobId: null, drawerJobId: null, selectedJobIds: [], undoStack: [],
    isScanning: false, scanStatusText: "", runningSourceIds: [], dataRevision: state.dataRevision + 1,
    ...(reset ? { filterSpec: { ...DEFAULT_FILTER_SPEC }, discoveredViewMode: "panes" as const,
      trackedViewMode: "kanban" as const, scraperMode: "simple" as const,
      scraperCategory: "workflows" as const, tooltipsEnabled: true,
      isFilterPanelOpen: false, unreadNotificationCount: 0, isNotificationOpen: false } : {}),
  }));
  await client.cancelQueries();
  // Keep the settings snapshot while refreshing it: resetting this query to
  // undefined triggers App's startup gate and unmounts the recovery receipt.
  await client.resetQueries({ predicate: query => query.queryKey[0] !== "settings" });
  await client.invalidateQueries({ queryKey: ["settings"] });
}

// Automatic retention uses the same cache/state boundary without touching settings
// or the user's filters. A deletion can invalidate an open job and undo entry.
export async function applyAutomaticRetention(client: QueryClient, status: RetentionStatus) {
  if (status.last_deleted_records <= 0) return;
  useUIStore.setState(state => ({
    selectedJobId: null, drawerJobId: null, selectedJobIds: [], undoStack: [],
    dataRevision: state.dataRevision + 1,
  }));
  await client.cancelQueries({ queryKey: ["jobs"] });
  await Promise.all(["jobs", "job_counts", "job_status_events", "metrics", "sources", "scheduler_status"].map(key =>
    client.invalidateQueries({ queryKey: [key] })
  ));
}
