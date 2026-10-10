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
import { ScraperView } from "./components/views/scraper/ScraperView";
import { SettingsView } from "./components/views/settings/SettingsView";
import { FilterPanel } from "./components/filters/FilterPanel";
import { matchesJob } from "./lib/filter";

export default function App() {
  const queryClient = useQueryClient();
  const {
    view,
    pushUndo,
    popUndo,
    filterSpec,
    setFilterSpec,
    resetFilterSpec,
    isFilterPanelOpen,
    setFilterPanelOpen,
  } = useUIStore();

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
  const {
    data: jobsPage,
    isLoading: jobsLoading,
    error: jobsError,
    refetch: refetchJobs,
  } = useQuery<Page<Job>>({
    queryKey: ["jobs"],
    queryFn: () => api.listJobs({ limit: 2000 }),
  });

  const {
    data: settings,
    isLoading: settingsLoading,
    error: settingsError,
    refetch: refetchSettings,
  } = useQuery({
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

  // 9. Separate discovered vs tracked jobs with active filter spec applied
  const filteredAllJobs = useMemo(() => {
    return allJobs.filter((j) => matchesJob(j, filterSpec));
  }, [allJobs, filterSpec]);

  const discoveredJobs = useMemo(() => {
    return filteredAllJobs.filter(
      (j) => j.status === "scraped" || j.status === "viewed"
    );
  }, [filteredAllJobs]);

  const trackedJobs = useMemo(() => {
    return filteredAllJobs.filter(
      (j) =>
        j.status === "applied" ||
        j.status === "interviewing" ||
        j.status === "offer" ||
        j.status === "rejected"
    );
  }, [filteredAllJobs]);

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

  // 10. Subsystem readiness gate
  const isInitializing = settingsLoading || jobsLoading;
  const initError = settingsError || jobsError;

  if (isInitializing) {
    return (
      <div className="w-screen h-screen flex flex-col items-center justify-center bg-[var(--color-paper)] text-[var(--color-ink)] select-none p-6">
        <div className="border border-[var(--color-ink)] p-8 max-w-md w-full bg-[var(--color-paper-2)]/30 space-y-6">
          <div className="flex items-center justify-between border-b border-[var(--color-ink)] pb-3">
            <span className="font-mono text-xs font-black tracking-widest text-[var(--color-ink)] uppercase">
              SCRAPE // CORE SYSTEM
            </span>
            <span className="font-mono text-[10px] tracking-wider text-[var(--color-ink-mute)] uppercase">
              BOOT-SEQ 01
            </span>
          </div>

          <div className="space-y-2">
            <div className="font-mono font-bold text-sm uppercase tracking-wider flex items-center gap-2">
              <span className="inline-block w-2 h-2 bg-[var(--color-ink)] animate-pulse" />
              INITIALIZING SUBSYSTEMS…
            </div>
            <p className="font-mono text-xs text-[var(--color-ink-mute)] leading-relaxed">
              Establishing IPC link with Scrape engine, loading database schema, and reading configuration registry.
            </p>
          </div>

          <div className="font-mono text-xs uppercase border-t border-[var(--color-ink)]/20 pt-3 space-y-2">
            <div className="flex justify-between items-center">
              <span className="text-[var(--color-ink-mute)]">SETTINGS REGISTRY:</span>
              <span className={settings ? "text-[var(--color-ink)] font-bold" : "animate-pulse font-bold text-[var(--color-ink-mute)]"}>
                {settings ? "[READY]" : "[SYNCING]"}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-[var(--color-ink-mute)]">JOB REPOSITORY:</span>
              <span className={jobsPage ? "text-[var(--color-ink)] font-bold" : "animate-pulse font-bold text-[var(--color-ink-mute)]"}>
                {jobsPage ? "[READY]" : "[SYNCING]"}
              </span>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (initError || !settings) {
    const errorMessage =
      initError instanceof Error
        ? initError.message
        : typeof initError === "string"
        ? initError
        : "Failed to load system settings configuration.";

    return (
      <div className="w-screen h-screen flex flex-col items-center justify-center bg-[var(--color-paper)] text-[var(--color-ink)] select-none p-6">
        <div className="border-2 border-[var(--color-ink)] p-8 max-w-lg w-full bg-[var(--color-paper)] space-y-6">
          <div className="border-b border-[var(--color-ink)] pb-3">
            <div className="font-mono text-[10px] font-black tracking-widest text-[var(--color-ink-mute)] uppercase">
              BOOT HALT // CRITICAL FAILURE
            </div>
            <div className="font-mono font-black text-sm uppercase tracking-wider mt-1 text-[var(--color-ink)]">
              SYSTEM INITIALIZATION FAILED
            </div>
          </div>

          <p className="font-mono text-xs text-[var(--color-ink-mute)] leading-relaxed">
            The application cannot start because the backend database or configuration service failed to initialize. User interaction is disabled to prevent data corruption.
          </p>

          <div className="bg-[var(--color-paper-2)] border border-[var(--color-ink)]/30 p-3 font-mono text-xs text-[var(--color-ink)] break-all">
            {errorMessage}
          </div>

          <div>
            <button
              onClick={() => {
                refetchSettings();
                refetchJobs();
              }}
              className="w-full py-2.5 px-4 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] font-mono text-xs font-bold uppercase hover:opacity-90 active:translate-y-[1px] cursor-pointer"
            >
              [↵] RETRY INITIALIZATION
            </button>
          </div>
        </div>
      </div>
    );
  }

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

      {view === "scraper" && (
        <ScraperView
          settings={settings}
          onUpdateSetting={(key, value) =>
            updateSettingMutation.mutate({ key, value })
          }
        />
      )}

      {view === "settings" && (
        <SettingsView
          settings={settings}
          onUpdateSetting={(key, value) =>
            updateSettingMutation.mutate({ key, value })
          }
        />
      )}

      {/* Slide-out Global Filter Panel */}
      <FilterPanel
        isOpen={isFilterPanelOpen}
        onClose={() => setFilterPanelOpen(false)}
        filterSpec={filterSpec}
        onUpdateFilter={setFilterSpec}
        onResetFilters={resetFilterSpec}
      />
    </Shell>
  );
}
