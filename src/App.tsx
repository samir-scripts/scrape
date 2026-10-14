import { openPostingUrl } from "./lib/postingUrl";
import { notifyError, useToastStore } from "./lib/toast";
import { useState, useEffect, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "./lib/api";
import { useUIStore } from "./lib/store";
import { setupAppEventListeners } from "./lib/events";
import type { Job } from "./bindings/Job";
import type { JobStatus } from "./bindings/JobStatus";
import type { JobFilter } from "./bindings/JobFilter";
import type { Page } from "./bindings/Page";
import type { Settings } from "./bindings/Settings";
import { Shell } from "./components/layout/Shell";
import { DiscoveredView } from "./components/views/discovered/DiscoveredView";
import { TrackedView } from "./components/views/tracked/TrackedView";
import { MetricsView } from "./components/views/metrics/MetricsView";
import { ScraperView } from "./components/views/scraper/ScraperView";
import { SettingsView } from "./components/views/settings/SettingsView";
import { FilterPanel } from "./components/filters/FilterPanel";
import { DeleteJobModal } from "./components/ui/DeleteJobModal";
import { useGlobalHotkeys } from "./hooks/useHotkeys";

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

  // 0. Register root-level application hotkeys (Tabs Q/E, Search /, Filters F, Escape)
  useGlobalHotkeys();

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

  const pageKey = JSON.stringify({ view, filterSpec });
  const [pageState, setPageState] = useState({ key: pageKey, offset: 0 });
  const offset = pageState.key === pageKey ? pageState.offset : 0;
  const pageSize = 50;
  const pageStatuses: JobStatus[] = view === "tracked" ? ["applied", "interviewing", "offer", "rejected"] : ["scraped", "viewed"];
  const effectiveFilter: Partial<JobFilter> = { ...filterSpec, sort: filterSpec.sort || (view === "tracked" ? "status_changed_at_desc" : "date_added_desc"), statuses: filterSpec.statuses?.length ? pageStatuses.filter((status) => filterSpec.statuses!.includes(status)) : pageStatuses, limit: pageSize, offset };
  const { data: counts } = useQuery({
    queryKey: ["job_counts"],
    queryFn: async () => {
      const [discovered, tracked] = await Promise.all([api.listJobs({ statuses: ["scraped", "viewed"], limit: 1 }), api.listJobs({ statuses: ["applied", "interviewing", "offer", "rejected"], limit: 1 })]);
      return { discovered: Number(discovered.total), tracked: Number(tracked.total) };
    },
  });
  // 2. Query application data
  const {
    data: jobsPage,
    isLoading: jobsLoading,
    error: jobsError,
    refetch: refetchJobs,
  } = useQuery<Page<Job>>({
    queryKey: ["jobs", pageKey, offset],
    queryFn: () => effectiveFilter.statuses?.length === 0 ? Promise.resolve({ items: [], total: 0n, limit: BigInt(pageSize), offset: BigInt(offset) }) : api.listJobs(effectiveFilter),
    enabled: view === "discovered" || view === "tracked",
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

  // 3. Sync UI theme, motion, tooltips, and font with settings
  useEffect(() => {
    try {
      const cachedFont = localStorage.getItem("scrape:font_family");
      if (cachedFont) {
        document.documentElement.setAttribute("data-font", cachedFont);
      }
    } catch {}
  }, []);

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
      document.documentElement.setAttribute(
        "data-font",
        settings.ui_font_family || "default"
      );
      try { localStorage.setItem("scrape:font_family", settings.ui_font_family || "default"); } catch {}
      if (typeof settings.ui_tooltips_enabled === "boolean") {
        useUIStore.getState().setTooltipsEnabled(settings.ui_tooltips_enabled);
      }
    }
  }, [settings]);

  // 3b. Check for application updates on startup if enabled
  useEffect(() => {
    if (settings?.app_auto_update) {
      api
        .checkAppUpdate()
        .then((res) => {
          if (res.available) {
            queryClient.invalidateQueries({ queryKey: ["notifications"] });
            api.countUnreadNotifications().then((count) => {
              useUIStore.getState().setUnreadNotificationCount(Number(count));
            });
          }
        })
        .catch((err) => {
          console.debug("Background update check skipped/failed:", err);
        });
    }
  }, [settings?.app_auto_update, queryClient]);

  const allJobs = useMemo(() => jobsPage?.items || [], [jobsPage]);

  const updateStatusMutation = useMutation({
    mutationFn: ({ jobId, status }: { jobId: string; status: JobStatus }) => api.updateJobStatus(jobId, status),
    onSuccess: (_job, { jobId, status }) => {
      const previous = allJobs.find((job) => job.id === jobId);
      if (previous) pushUndo({ jobId, previousStatus: previous.status });
      if (status === "applied" && view === "discovered") useToastStore.getState().addToast({type:"success",message:"Application added to Tracked.",action:{label:"View in Tracked",run:()=>useUIStore.getState().setView("tracked")}});
    },
    onError: (error) => notifyError(String(error), "STATUS UPDATE FAILED"),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
      queryClient.invalidateQueries({ queryKey: ["job_status_events"] });
    },
  });

  // 5. Notes mutation
  const updateNotesMutation = useMutation({
    onError: (error) => notifyError(String(error), "NOTES SAVE FAILED"),
    mutationFn: ({ jobId, notes }: { jobId: string; notes: string }) =>
      api.updateJobNotes(jobId, notes),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });

  const toggleFavoriteMutation = useMutation({
    mutationFn: (jobId: string) => api.toggleJobFavorite(jobId),
    onError: (error) => notifyError(String(error), "FAVORITE UPDATE FAILED"),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["metrics"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    },
  });

  // 6. Settings mutation with optimistic updates
  const updateSettingMutation = useMutation({
    mutationFn: ({ key, value }: { key: string; value: unknown }) =>
      api.updateSetting(key, value),
    onMutate: async ({ key, value }) => {
      await queryClient.cancelQueries({ queryKey: ["settings"] });
      const previousSettings = queryClient.getQueryData<Settings>(["settings"]);
      if (previousSettings) {
        const fieldKey = key.replace(/\./g, "_") as keyof Settings;
        queryClient.setQueryData<Settings>(["settings"], {
          ...previousSettings,
          [fieldKey]: value,
        });
      }
      return { previousSettings };
    },
    onError: (err, _vars, context) => {
      notifyError(String(err), "SETTING UPDATE FAILED");
      if (context?.previousSettings) {
        queryClient.setQueryData(["settings"], context.previousSettings);
      }
    },
    onSuccess: (newSettings) => {
      queryClient.setQueryData(["settings"], newSettings);
      queryClient.invalidateQueries({ queryKey: ["scheduler_status"] });
    },
  });

  const [jobPendingDelete, setJobPendingDelete] = useState<Job | null>(null);
  const { data: removalCopies } = useQuery({
    queryKey: ["removal_copies", jobPendingDelete?.id],
    queryFn: () => api.postingSavedCopies(jobPendingDelete!.id),
    enabled: !!jobPendingDelete,
  });
  const deleteJobMutation = useMutation({
    mutationFn: ({ id, block }: { id: string; block: boolean }) => api.removeJob(id, block),
    onSuccess: (result) => {
      setJobPendingDelete(null);
      useToastStore.getState().addToast({ type: "success", message: `${result.copies} saved ${result.copies === 1 ? "copy" : "copies"} removed.`, duration: 10000,
        action: { label: "Undo", run: () => { api.restorePosting(result.identity).then(() => {
          queryClient.invalidateQueries({ queryKey: ["jobs"] });
          queryClient.invalidateQueries({ queryKey: ["metrics"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
          queryClient.invalidateQueries({ queryKey: ["removed_postings"] });
        }).catch((error) => notifyError(String(error), "RESTORE FAILED")); } } });
    },
    onError: (error) => notifyError(String(error), "REMOVAL FAILED"),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["metrics"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
      queryClient.invalidateQueries({ queryKey: ["removed_postings"] });
    },
  });
  const handleRequestDelete = (job: Job) => setJobPendingDelete(job);
  const handleConfirmDelete = (block: boolean) => {
    if (jobPendingDelete) deleteJobMutation.mutate({ id: jobPendingDelete.id, block });
  };

  // 7. Undo handler
  const handleUndo = async () => {
    const item = popUndo();
    if (!item) return;
    try {
      await api.updateJobStatus(item.jobId, item.previousStatus);
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    } catch (e) {
      notifyError(String(e), "UNDO FAILED");
    }
  };

  // 8. Open URL helper (Tauri native opener with resilient browser fallback)
  const handleOpenUrl = async (url: string) => {
    try { await openPostingUrl(url); } catch (error) { notifyError(String(error), "OPEN POSTING FAILED"); }
  };

  const discoveredJobs = view === "discovered" ? allJobs : [];
  const trackedJobs = view === "tracked" ? allJobs : [];
  useEffect(() => {
    const total = Number(jobsPage?.total || 0);
    if (offset > 0 && jobsPage && offset >= total) setPageState({ key: pageKey, offset: Math.max(0, Math.floor((total - 1) / pageSize) * pageSize) });
  }, [jobsPage, offset, pageKey]);

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
  const isInitializing = settingsLoading;
  const initError = settingsError;

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
              Loading your settings and saved jobs.
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
      discoveredCount={counts?.discovered ?? 0}
      trackedCount={counts?.tracked ?? 0}
      nextRunText={nextRunText}
    >
      {(view === "discovered" || view === "tracked") && <div className="flex flex-wrap items-center gap-3 border-b px-4 py-2 text-sm">
        <button type="button" disabled={offset === 0 || jobsLoading} onClick={() => setPageState({ key: pageKey, offset: Math.max(0, offset - pageSize) })} className="border px-3 py-1">Previous page</button>
        <span role="status">{jobsLoading ? "Loading jobs…" : `${Number(jobsPage?.total || 0) === 0 ? 0 : offset + 1}–${Math.min(offset + pageSize, Number(jobsPage?.total || 0))} of ${jobsPage?.total ?? 0}`}</span>
        <button type="button" disabled={jobsLoading || offset + pageSize >= Number(jobsPage?.total || 0)} onClick={() => setPageState({ key: pageKey, offset: offset + pageSize })} className="border px-3 py-1">Next page</button>
        <label>Sort <select aria-label="Sort jobs" value={filterSpec.sort || (view === "tracked" ? "status_changed_at_desc" : "date_added_desc")} onChange={(event) => setFilterSpec({ sort: event.target.value as JobFilter["sort"] })} className="border bg-[var(--color-paper)] p-1">
          <option value="status_changed_at_desc">Recent stage activity</option><option value="date_added_desc">Newest discoveries</option><option value="date_added_asc">Oldest discoveries</option><option value="date_posted_desc">Newest postings</option><option value="company_asc">Company</option><option value="title_asc">Title</option><option value="salary_desc">Derived annual pay</option><option value="favorited_desc">Favorites first</option>
        </select></label>
        {jobsError && <p role="alert">Could not load this page: {String(jobsError)} <button onClick={() => refetchJobs()}>Retry</button></p>}
      </div>}
      {view === "discovered" && !jobsLoading && (
        <DiscoveredView
          jobs={discoveredJobs}
          onUpdateStatus={(jobId, status) =>
            updateStatusMutation.mutate({ jobId, status })
          }
          onUpdateNotes={(jobId, notes) =>
            updateNotesMutation.mutateAsync({ jobId, notes })
          }
          onOpenUrl={handleOpenUrl}
          onUndo={handleUndo}
          onToggleFavorite={(jobId) => toggleFavoriteMutation.mutate(jobId)}
          onRequestDelete={handleRequestDelete}
        />
      )}

      {view === "tracked" && !jobsLoading && (
        <TrackedView
          jobs={trackedJobs}
          onUpdateStatus={(jobId, status) =>
            updateStatusMutation.mutate({ jobId, status })
          }
          onUpdateNotes={(jobId, notes) =>
            updateNotesMutation.mutateAsync({ jobId, notes })
          }
          onOpenUrl={handleOpenUrl}
          onUndo={handleUndo}
          onToggleFavorite={(jobId) => toggleFavoriteMutation.mutate(jobId)}
          onRequestDelete={handleRequestDelete}
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

      {/* Delete Job Confirmation Modal */}
      <DeleteJobModal
        key={jobPendingDelete?.id || "closed"}
        isOpen={jobPendingDelete !== null}
        job={jobPendingDelete}
        copies={removalCopies}
        pending={deleteJobMutation.isPending}
        onClose={() => setJobPendingDelete(null)}
        onConfirm={handleConfirmDelete}
      />
    </Shell>
  );
}
