import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";
import { useUIStore } from "./store";

interface ScrapeProgressPayload {
  source_id?: string;
  fetched?: number;
  matched?: number;
  error?: string;
}

export async function setupAppEventListeners(
  queryClient: QueryClient
): Promise<() => void> {
  const unlisteners: UnlistenFn[] = [];

  const unlistenJobs = await listen("jobs://changed", () => {
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    queryClient.invalidateQueries({ queryKey: ["job_status_events"] });
  });
  unlisteners.push(unlistenJobs);

  const unlistenSettings = await listen("settings://changed", () => {
    queryClient.invalidateQueries({ queryKey: ["settings"] });
    queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
  });
  unlisteners.push(unlistenSettings);

  const unlistenStarted = await listen("scrape://started", () => {
    useUIStore.getState().setScanning(true, "STARTING SCRAPE RUN...");
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

  const unlistenFinished = await listen("scrape://finished", () => {
    useUIStore.getState().setScanning(false, "");
    queryClient.invalidateQueries({ queryKey: ["scrape_runs"] });
    queryClient.invalidateQueries({ queryKey: ["sources"] });
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
  });
  unlisteners.push(unlistenFinished);

  return () => {
    unlisteners.forEach((fn) => fn());
  };
}
