import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { ScrapeRun } from "../../../bindings/ScrapeRun";
import type { ScrapeLogEntry } from "../../../bindings/ScrapeLogEntry";
import { api } from "../../../lib/api";
import { useUIStore } from "../../../lib/store";
import { EmptyState } from "../../ui/EmptyState";

export function LogsSettings() {
  const queryClient = useQueryClient();
  const { setScraperCategory, setView } = useUIStore();
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [filterMode, setFilterMode] = useState<"all" | "errors">("all");

  // Fetch runs list
  const {
    data: runs = [],
    isLoading: runsLoading,
  } = useQuery({
    queryKey: ["scrape_runs"],
    queryFn: () => api.listScrapeRuns(50),
    refetchInterval: 5000,
  });

  // Sort runs newest first (started_at DESC)
  const sortedRuns = useMemo(() => {
    return [...runs].sort((a, b) => {
      return new Date(b.started_at).getTime() - new Date(a.started_at).getTime();
    });
  }, [runs]);

  // Determine if a run requires attention (failed, partial, or has an error message)
  const isRunError = (run: ScrapeRun): boolean => {
    return run.status === "failed" || run.status === "partial" || !!run.error;
  };

  // Filter runs based on toggle
  const displayedRuns = useMemo(() => {
    if (filterMode === "errors") {
      return sortedRuns.filter(isRunError);
    }
    return sortedRuns;
  }, [sortedRuns, filterMode]);

  // Keep selection valid
  const effectiveSelectedId = selectedRunId || (displayedRuns[0]?.id ?? null);
  const selectedRun = useMemo(() => {
    return runs.find((r) => r.id === effectiveSelectedId) || null;
  }, [runs, effectiveSelectedId]);

  // Fetch logs for selected run
  const {
    data: logs = [],
    isLoading: logsLoading,
  } = useQuery({
    queryKey: ["scrape_logs", effectiveSelectedId],
    queryFn: () => (effectiveSelectedId ? api.getScrapeLogs(effectiveSelectedId) : Promise.resolve([])),
    enabled: !!effectiveSelectedId,
    refetchInterval: 3000,
  });

  // Manual trigger mutation
  const runNowMutation = useMutation({
    mutationFn: api.runScrapeNow,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["scrape_runs"] });
    },
  });

  const formatDateTime = (isoString?: string | null) => {
    if (!isoString) return "—";
    try {
      const d = new Date(isoString);
      return `${d.toLocaleDateString([], { month: "2-digit", day: "2-digit" })} ${d.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      })}`;
    } catch {
      return isoString;
    }
  };

  const formatDuration = (startStr: string, endStr?: string | null) => {
    if (!endStr) return "RUNNING…";
    try {
      const start = new Date(startStr).getTime();
      const end = new Date(endStr).getTime();
      const diffMs = Math.max(0, end - start);
      const secs = (diffMs / 1000).toFixed(1);
      return `${secs}s`;
    } catch {
      return "—";
    }
  };

  return (
    <div className="flex flex-col gap-5 w-full h-full select-none font-mono">
      {/* Header Banner */}
      <div className="flex justify-between items-start pb-2 border-b border-[var(--color-ink)]">
        <div>
          <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
            LOGS
          </h2>
        </div>

        <div className="flex items-center gap-2">
          {/* Filter toggle */}
          <div className="flex border border-[var(--color-ink)] bg-[var(--color-paper)]">
            <button
              onClick={() => setFilterMode("all")}
              className={`px-2.5 py-1 text-xs font-bold uppercase cursor-pointer ${
                filterMode === "all"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              ALL RUNS ({sortedRuns.length})
            </button>
            <button
              onClick={() => setFilterMode("errors")}
              className={`px-2.5 py-1 text-xs font-bold uppercase flex items-center gap-1.5 cursor-pointer ${
                filterMode === "errors"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              <span className="inline-block w-2.5 h-2.5 bg-red-600 border border-red-700" />
              ERRORS ({sortedRuns.filter(isRunError).length})
            </button>
          </div>

          {/* Trigger Now button */}
          <button
            onClick={() => runNowMutation.mutate()}
            disabled={runNowMutation.isPending}
            className="px-3 py-1 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-90 active:translate-y-[1px] text-xs font-bold uppercase cursor-pointer disabled:opacity-50"
          >
            {runNowMutation.isPending ? "STARTING…" : "RUN SCRAPER NOW"}
          </button>
        </div>
      </div>

      {/* Master-Detail Layout */}
      <div className="grid grid-cols-12 gap-4 flex-1 min-h-[480px]">
        {/* Left Column: Runs List */}
        <div className="col-span-5 border border-[var(--color-ink)] bg-[var(--color-paper)] flex flex-col overflow-hidden">
          <div className="px-3 py-2 bg-[var(--color-paper-2)]/40 border-b border-[var(--color-ink)] flex justify-between items-center text-[11px] font-bold uppercase tracking-wider text-[var(--color-ink)]">
            <span>RUNS ({displayedRuns.length})</span>
            <span className="text-[var(--color-ink-mute)]">STATUS · JOBS</span>
          </div>

          <div className="flex-1 overflow-y-auto divide-y divide-[var(--color-ink)]/30">
            {runsLoading && runs.length === 0 ? (
              <div className="p-6 text-center text-xs text-[var(--color-ink-mute)]">
                LOADING RUN HISTORY…
              </div>
            ) : displayedRuns.length === 0 ? (
              <div className="p-6 text-center text-xs text-[var(--color-ink-mute)]">
                {filterMode === "errors"
                  ? "NO RUNS WITH ERRORS DETECTED."
                  : "NO SCRAPE RUNS RECORDED YET."}
              </div>
            ) : (
              displayedRuns.map((run) => {
                const isSelected = run.id === effectiveSelectedId;
                const hasError = isRunError(run);

                return (
                  <button
                    key={run.id}
                    onClick={() => setSelectedRunId(run.id)}
                    className={`w-full text-left p-3 flex flex-col gap-1 cursor-pointer transition-none ${
                      isSelected
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                        : "hover:bg-[var(--color-paper-2)] text-[var(--color-ink)]"
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <div className="flex items-center gap-2">
                        {/* Red square error marker */}
                        {hasError ? (
                          <span
                            className="inline-block w-2.5 h-2.5 bg-red-600 border border-red-700 shrink-0"
                            title="Error occurred during this scrape run"
                          />
                        ) : (
                          <span
                            className="inline-block w-2.5 h-2.5 bg-[var(--color-lime)] border border-[var(--color-lime)] shrink-0"
                            title="Completed successfully"
                          />
                        )}
                        <span className="font-bold">
                          {formatDateTime(run.started_at)}
                        </span>
                      </div>
                      <span className="text-[10px] uppercase font-bold tracking-wider opacity-80">
                        {run.trigger.toUpperCase()}
                      </span>
                    </div>

                    <div className="flex items-center justify-between text-[11px] opacity-80 pt-0.5">
                      <span className="uppercase">
                        STATUS:{" "}
                        <strong
                          className={
                            hasError
                              ? isSelected
                                ? "text-red-300"
                                : "text-red-600 font-bold"
                              : ""
                          }
                        >
                          {run.status.toUpperCase()}
                        </strong>
                      </span>
                      <span>
                        +{Number(run.jobs_new)} NEW / {Number(run.jobs_found)} FOUND
                      </span>
                    </div>
                  </button>
                );
              })
            )}
          </div>
        </div>

        {/* Right Column: Selected Run Inspector */}
        <div className="col-span-7 border border-[var(--color-ink)] bg-[var(--color-paper)] flex flex-col overflow-hidden">
          {selectedRun ? (
            <>
              {/* Run Meta Header */}
              <div className="p-4 bg-[var(--color-paper-2)]/30 border-b border-[var(--color-ink)] flex flex-col gap-2">
                <div className="flex justify-between items-start">
                  <div>
                    <div className="flex items-center gap-2">
                      {isRunError(selectedRun) ? (
                        <span
                          className="inline-block w-3 h-3 bg-red-600 border border-red-700"
                          title="REQUIRES ATTENTION"
                        />
                      ) : (
                        <span className="inline-block w-3 h-3 bg-[var(--color-lime)]" />
                      )}
                      <span className="font-bold text-sm uppercase text-[var(--color-ink)]">
                        RUN: {selectedRun.id.slice(0, 8)}…
                      </span>
                    </div>
                    <span className="text-[11px] text-[var(--color-ink-mute)]">
                      STARTED: {formatDateTime(selectedRun.started_at)} · DURATION:{" "}
                      {formatDuration(selectedRun.started_at, selectedRun.finished_at)}
                    </span>
                  </div>

                  <div className="text-right">
                    <span
                      className={`text-xs font-bold uppercase px-2 py-0.5 border ${
                        isRunError(selectedRun)
                          ? "border-red-600 bg-red-100 text-red-700"
                          : "border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)]"
                      }`}
                    >
                      {selectedRun.status.toUpperCase()}
                    </span>
                  </div>
                </div>

                {/* Prominent Error Banner if run failed */}
                {selectedRun.error && (
                  <div className="p-2.5 border border-red-600 bg-red-50 text-red-800 text-xs flex flex-col gap-1">
                    <span className="font-bold uppercase flex items-center gap-1.5">
                      <span className="inline-block w-2.5 h-2.5 bg-red-600" />
                      ERROR REQUIRING ATTENTION:
                    </span>
                    <span className="break-all font-mono">
                      {selectedRun.error}
                    </span>
                  </div>
                )}
              </div>

              {/* Log Event Stream */}
              <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
                <div className="flex justify-between items-center pb-1 border-b border-[var(--color-ink)]/20 text-[10px] text-[var(--color-ink-mute)] font-bold uppercase tracking-wider">
                  <span>EXECUTION LOG EVENTS ({logs.length})</span>
                  <span>TIMESTAMP · LEVEL</span>
                </div>

                {logsLoading && logs.length === 0 ? (
                  <div className="p-4 text-center text-xs text-[var(--color-ink-mute)]">
                    LOADING LOG EVENTS…
                  </div>
                ) : logs.length === 0 ? (
                  <div className="p-4 text-center text-xs text-[var(--color-ink-mute)]">
                    NO LOG EVENTS RECORDED FOR THIS RUN.
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5 font-mono text-[11px]">
                    {logs.map((log: ScrapeLogEntry) => {
                      const isError = log.level === "error";
                      const isWarn = log.level === "warn";

                      return (
                        <div
                          key={log.id}
                          className={`p-2 border ${
                            isError
                              ? "border-red-600 bg-red-50/50 text-red-900"
                              : isWarn
                              ? "border-amber-600 bg-amber-50/50 text-amber-900"
                              : "border-[var(--color-ink)]/30 bg-[var(--color-paper-2)]/20 text-[var(--color-ink)]"
                          }`}
                        >
                          <div className="flex items-center justify-between text-[10px] mb-1 opacity-80">
                            <div className="flex items-center gap-1.5 font-bold uppercase">
                              {isError && (
                                <span className="inline-block w-2 h-2 bg-red-600" />
                              )}
                              <span>[{log.level.toUpperCase()}]</span>
                              {log.source_id && (
                                <span>SOURCE: {log.source_id.slice(0, 8)}</span>
                              )}
                            </div>
                            <span>
                              {new Date(log.timestamp).toLocaleTimeString([], {
                                hour: "2-digit",
                                minute: "2-digit",
                                second: "2-digit",
                                hour12: false,
                              })}
                            </span>
                          </div>

                          <div className="break-all whitespace-pre-wrap leading-relaxed">
                            {log.message}
                          </div>

                          {/* Quick Edit Source link if error is attached to a source */}
                          {isError && log.source_id && (
                            <div className="mt-2 pt-1.5 border-t border-red-300 flex justify-end">
                              <button
                                onClick={() => {
                                  setView("scraper");
                                  setScraperCategory("sources");
                                }}
                                className="px-2 py-0.5 border border-red-600 bg-white text-red-700 hover:bg-red-600 hover:text-white text-[10px] font-bold uppercase cursor-pointer"
                              >
                                [EDIT SOURCE IN SCRAPER →]
                              </button>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center p-8">
              <EmptyState
                title="NO RUN SELECTED"
                description="SELECT A SCRAPE RUN FROM THE LIST ON THE LEFT TO VIEW EXECUTION LOGS."
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
