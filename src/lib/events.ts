import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";
import { useUIStore } from "./store";

interface ScrapeProgressPayload {
  source_id?: string;
  fetched?: number;
  matched?: number;
  error?: string;
}

interface ScrapeStartedPayload {
  run_id?: string;
  source_id?: string;
}

interface ScrapeSourceFinishedPayload {
  source_id?: string;
  run?: unknown;
  duration_ms?: number;
}

export async function setupAppEventListeners(
  queryClient: QueryClient
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }
  const unlisteners: UnlistenFn[] = [];

  const unlistenJobs = await listen("jobs://changed", () => {
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    queryClient.invalidateQueries({ queryKey: ["job_status_events"] });
    queryClient.invalidateQueries({ queryKey: ["metrics"] });
  });
  unlisteners.push(unlistenJobs);

  const unlistenSettings = await listen("settings://changed", () => {
    queryClient.invalidateQueries({ queryKey: ["settings"] });
    queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
  });
  unlisteners.push(unlistenSettings);

  const unlistenStarted = await listen<ScrapeStartedPayload>("scrape://started", (event) => {
    useUIStore.getState().setScanning(true, "STARTING SCRAPE RUN...");
    if (event.payload?.source_id) {
      useUIStore.getState().setSourceRunning(event.payload.source_id, true);
    }
    queryClient.invalidateQueries({ queryKey: ["scrape_runs"] });
    queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
  });
  unlisteners.push(unlistenStarted);

  const unlistenProgress = await listen<ScrapeProgressPayload>(
    "scrape://progress",
    (event) => {
      const p = event.payload;
      if (p.error) {
        useUIStore
          .getState()
          .setScanning(true, `ERROR ON SOURCE: ${p.error.slice(0, 30)}`);
      } else if (p.source_id) {
        useUIStore.getState().setSourceRunning(p.source_id, true);
        useUIStore
          .getState()
          .setScanning(
            true,
            `FETCHED ${p.fetched ?? 0} · MATCHED ${p.matched ?? 0}`
          );
      }
    }
  );
  unlisteners.push(unlistenProgress);

  const unlistenSourceFinished = await listen<ScrapeSourceFinishedPayload>(
    "scrape://source_finished",
    (event) => {
      if (event.payload?.source_id) {
        useUIStore.getState().setSourceRunning(event.payload.source_id, false);
      }
      queryClient.invalidateQueries({ queryKey: ["sources"] });
      queryClient.invalidateQueries({ queryKey: ["scrape_runs"] });
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["job_counts"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
      queryClient.invalidateQueries({ queryKey: ["metrics"] });
    }
  );
  unlisteners.push(unlistenSourceFinished);

  const unlistenFinished = await listen("scrape://finished", () => {
    useUIStore.getState().setScanning(false, "");
    useUIStore.getState().clearRunningSources();
    queryClient.invalidateQueries({ queryKey: ["scrape_runs"] });
    queryClient.invalidateQueries({ queryKey: ["sources"] });
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
    queryClient.invalidateQueries({ queryKey: ["metrics"] });
  });
  unlisteners.push(unlistenFinished);

  return () => {
    unlisteners.forEach((fn) => fn());
  };
}
