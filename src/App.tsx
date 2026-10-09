import { useEffect, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./lib/api";
import { useUIStore } from "./lib/store";
import { setupAppEventListeners } from "./lib/events";
import type { Job } from "./bindings/Job";
import type { JobStatus } from "./bindings/JobStatus";
import type { Page } from "./bindings/Page";
import { Shell } from "./components/layout/Shell";
import { DiscoveredView } from "./components/views/discovered/DiscoveredView";
import { TrackedView } from "./components/views/tracked/TrackedView";
import { MetricsView } from "./components/views/metrics/MetricsView";
import { SettingsView } from "./components/views/settings/SettingsView";

export default function App() {
  const queryClient = useQueryClient();
  const { view, pushUndo, popUndo } = useUIStore();

  // 1. Subscribe to backend events on mount
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    setupAppEventListeners(queryClient).then((unlisten) => {
      cleanup = unlisten;
    });
    return () => {
      if (cleanup) cleanup();
    };
  }, [queryClient]);

  // 2. Query application data
  const { data: jobsPage } = useQuery<Page<Job>>({
    queryKey: ["jobs"],
    queryFn: () => api.listJobs({ limit: 2000 }),
  });

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: api.getSettings,
  });

  const { data: schedulerStatus } = useQuery({
    queryKey: ["scheduler_status"],
    queryFn: api.getSchedulerStatus,
  });

  // 3. Sync UI theme and motion with settings
  useEffect(() => {
    if (settings) {
      document.documentElement.setAttribute(
        "data-theme",
        settings.ui_theme || "paper"
      );
      document.documentElement.setAttribute(
        "data-reduce-motion",
        settings.ui_reduce_motion ? "true" : "false"
      );
    }
  }, [settings]);

  const allJobs = useMemo(() => jobsPage?.items || [], [jobsPage]);

  // 4. Status transition mutation with optimistic updates
  const updateStatusMutation = useMutation({
    mutationFn: async ({
      jobId,
      status,
    }: {
      jobId: string;
      status: JobStatus;
    }) => {
      return api.updateJobStatus(jobId, status);
    },
    onMutate: async ({ jobId, status }) => {
      await queryClient.cancelQueries({ queryKey: ["jobs"] });
      const previousJobs = queryClient.getQueryData<Page<Job>>(["jobs"]);

      if (previousJobs) {
        const targetJob = previousJobs.items.find((j) => j.id === jobId);
        if (targetJob) {
          // Push to undo stack
          pushUndo({
            jobId: targetJob.id,
            previousStatus: targetJob.status,
          });

          // Optimistically update
          queryClient.setQueryData<Page<Job>>(["jobs"], {
            ...previousJobs,
            items: previousJobs.items.map((j) =>
              j.id === jobId ? { ...j, status } : j
            ),
          });
        }
      }

      return { previousJobs };
    },
    onError: (_err, _vars, context) => {
      if (context?.previousJobs) {
        queryClient.setQueryData(["jobs"], context.previousJobs);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["job_status_events"] });
    },
  });

  // 5. Notes mutation
  const updateNotesMutation = useMutation({
    mutationFn: ({ jobId, notes }: { jobId: string; notes: string }) =>
      api.updateJobNotes(jobId, notes),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });

  // 6. Settings mutation
  const updateSettingMutation = useMutation({
    mutationFn: ({ key, value }: { key: string; value: unknown }) =>
      api.updateSetting(key, value),
    onSuccess: (newSettings) => {
      queryClient.setQueryData(["settings"], newSettings);
      queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
    },
  });

  // 7. Undo handler
  const handleUndo = async () => {
    const item = popUndo();
    if (!item) return;
    try {
      await api.updateJobStatus(item.jobId, item.previousStatus);
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    } catch (e) {
      console.error("Undo error:", e);
    }
  };

  // 8. Open URL helper
  const handleOpenUrl = async (url: string) => {
    if (!url) return;
    try {
      await openUrl(url);
    } catch (e) {
      console.error("Failed to open URL in browser:", e);
    }
  };

  // 9. Separate discovered vs tracked jobs
  const discoveredJobs = useMemo(() => {
    return allJobs.filter(
      (j) => j.status === "scraped" || j.status === "viewed"
    );
  }, [allJobs]);

  const trackedJobs = useMemo(() => {
    return allJobs.filter(
      (j) =>
        j.status === "applied" ||
        j.status === "interviewing" ||
        j.status === "offer" ||
        j.status === "rejected"
    );
  }, [allJobs]);

  const nextRunText = useMemo(() => {
    if (!schedulerStatus?.enabled) return "PAUSED";
    if (!schedulerStatus?.next_run_at) return "SCHEDULED";
    try {
      const d = new Date(schedulerStatus.next_run_at);
      return `NEXT ${d.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })}`;
    } catch {
      return "SCHEDULED";
    }
  }, [schedulerStatus]);

  return (
    <Shell
      discoveredCount={discoveredJobs.length}
      trackedCount={trackedJobs.length}
      nextRunText={nextRunText}
    >
      {view === "discovered" && (
        <DiscoveredView
          jobs={discoveredJobs}
          onUpdateStatus={(jobId, status) =>
            updateStatusMutation.mutate({ jobId, status })
          }
          onUpdateNotes={(jobId, notes) =>
            updateNotesMutation.mutate({ jobId, notes })
          }
          onOpenUrl={handleOpenUrl}
          onUndo={handleUndo}
        />
      )}

      {view === "tracked" && (
        <TrackedView
          jobs={trackedJobs}
          onUpdateStatus={(jobId, status) =>
            updateStatusMutation.mutate({ jobId, status })
          }
          onUpdateNotes={(jobId, notes) =>
            updateNotesMutation.mutate({ jobId, notes })
          }
          onOpenUrl={handleOpenUrl}
          onUndo={handleUndo}
        />
      )}

      {view === "metrics" && <MetricsView />}

      {view === "settings" && (
        <SettingsView
          settings={settings || null}
          onUpdateSetting={(key, value) =>
            updateSettingMutation.mutate({ key, value })
          }
        />
      )}
    </Shell>
  );
}
