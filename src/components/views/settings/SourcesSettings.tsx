import { sourceNeedsAttention, runStatusBadge, runStatusLabel } from "../../../lib/status";
import { ScanDiagnostics } from "./ScanDiagnostics";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import { WorkflowImportButton } from "./WorkflowImportButton";
import { IconButton } from "../../ui/IconButton";
import { Icon } from "../../ui/Icon";
import { isRecordedMode } from "../../../lib/runtime";
import { sourceToWorkflowSpec } from "../../../lib/yamlWorkflow";
import { useState, useEffect, useMemo } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { Source } from "../../../bindings/Source";
import type { ScrapeRun } from "../../../bindings/ScrapeRun";
import type { ScrapeLogEntry } from "../../../bindings/ScrapeLogEntry";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import type { Settings } from "../../../bindings/Settings";
import { api } from "../../../lib/api";
import { listen } from "@tauri-apps/api/event";
import { useUIStore } from "../../../lib/store";
import { formatDateTime, formatTime } from "../../../lib/formatters";
import { Button } from "../../ui/Button";
import { Toggle } from "../../ui/Toggle";
import { Input } from "../../ui/Input";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Tooltip } from "../../ui/Tooltip";
import { Select } from "../../ui/Select";
import { AddSourceModal } from "./AddSourceModal";
import { notifyError, notifySuccess, notifyInfo } from "../../../lib/toast";
import { getWorkflowStatus } from "../../../lib/status";
import { ActionMenu } from "../../ui/ActionMenu";

type FilterTab = "all" | "ats" | "custom" | "errors";

function workflowHasRunnableSources(source: Source): boolean {
  if (!source.enabled) return false;
  const config = source.config as Record<string, unknown> | null;
  const spec = config?.__workflow_spec as { sources?: Array<{ enabled?: boolean }> } | undefined;
  return spec?.sources ? spec.sources.some(child => child.enabled === true) : true;
}

function workflowTimezone(source: Source, settings?: Settings): string {
  const config = source.config && typeof source.config === "object" && !Array.isArray(source.config) ? source.config : {};
  const schedule = config.__schedule && typeof config.__schedule === "object" && !Array.isArray(config.__schedule) ? config.__schedule : {};
  const timezone = typeof config.__cron_timezone === "string" ? config.__cron_timezone : schedule.timezone;
  return typeof timezone === "string" && timezone || settings?.system_timezone || Intl.DateTimeFormat().resolvedOptions().timeZone;
}

interface SourcesSettingsProps {
  settings?: Settings;
}

export function SourcesSettings({ settings }: SourcesSettingsProps = {}) {
  const queryClient = useQueryClient();
  const [sources, setSources] = useState<Source[]>([]);
  const [sourceLoadError, setSourceLoadError] = useState("");
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<FilterTab>("all");

  // Modals & drawers
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [importedSpec, setImportedSpec] = useState<WorkflowSpec | null>(null);
  const [runningAll, setRunningAll] = useState(false);
  const [editingSource, setEditingSource] = useState<Source | null>(null);

  // Per-job execution state
  const [runningSourceId, setRunningSourceId] = useState<string | null>(null);
  const [testingSourceId, setTestingSourceId] = useState<string | null>(null);
  const [testEvents, setTestEvents] = useState<WorkerEvent[] | null>(null);

  // Inspector & Consolidated Logs Dock state
  const [inspectedSourceId, setInspectedSourceId] = useState<string | null>(null);
  const inspectedSource = sources.find(source => source.id === inspectedSourceId) ?? null;
  const [sourceRuns, setSourceRuns] = useState<ScrapeRun[]>([]);
  const [isLoadingRuns, setIsLoadingRuns] = useState(false);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [runLogs, setRunLogs] = useState<ScrapeLogEntry[]>([]);
  const [isLoadingLogs, setIsLoadingLogs] = useState(false);
  const [logFilter, setLogFilter] = useState<"all" | "successes" | "errors">("all");

  const { setView, setFilterSpec, runningSourceIds, isScanning, scanStatusText } = useUIStore();

  const fetchSources = () => {
    api.listSources().then(sources => { setSources(sources); setSourceLoadError(""); }).catch(error => setSourceLoadError(String(error)));
  };

  useEffect(() => {
    fetchSources();
  }, []);

  const refreshInspectedRunsAndLogs = async (sourceId: string) => {
    try {
      setIsLoadingRuns(true);
      const runs = await api.listSourceRuns(sourceId, 10);
      setSourceRuns(runs);
      if (runs.length > 0) {
        const targetRunId = runs[0].id;
        setSelectedRunId(targetRunId);
        setIsLoadingLogs(true);
        const logs = await api.getScrapeLogs(targetRunId).catch(() => []);
        setRunLogs(logs);
        setIsLoadingLogs(false);
      } else {
        setSelectedRunId(null);
        setRunLogs([]);
      }
    } catch {
      setSourceRuns([]);
      setSelectedRunId(null);
      setRunLogs([]);
    } finally {
      setIsLoadingRuns(false);
    }
  };

  // Fetch runs when inspecting a source
  useEffect(() => {
    if (!inspectedSource) {
      setSourceRuns([]);
      setSelectedRunId(null);
      setRunLogs([]);
      return;
    }
    refreshInspectedRunsAndLogs(inspectedSource.id);
  }, [inspectedSource]);

  // Real-time event listener for scrape progress and completion
  useEffect(() => {
    let unlistenSourceFinished: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;

    listen<{ source_id?: string }>("scrape://source_finished", (event) => {
      fetchSources();
      if (inspectedSource && event.payload?.source_id === inspectedSource.id) {
        refreshInspectedRunsAndLogs(inspectedSource.id);
      }
    })
      .then((un) => {
        unlistenSourceFinished = un;
      })
      .catch(() => {});

    listen("scrape://finished", () => {
      fetchSources();
      if (inspectedSource) {
        refreshInspectedRunsAndLogs(inspectedSource.id);
      }
    })
      .then((un) => {
        unlistenFinished = un;
      })
      .catch(() => {});

    return () => {
      unlistenSourceFinished?.();
      unlistenFinished?.();
    };
  }, [inspectedSource]);

  // Fetch logs when switching selected run manually
  useEffect(() => {
    if (!selectedRunId) {
      setRunLogs([]);
      return;
    }
    setIsLoadingLogs(true);
    api.getScrapeLogs(selectedRunId)
      .then(setRunLogs)
      .catch(() => setRunLogs([]))
      .finally(() => setIsLoadingLogs(false));
  }, [selectedRunId]);

  const handleToggleSource = async (source: Source, enabled: boolean) => {
    try { await api.updateSource({ id: source.id, enabled }); fetchSources(); }
    catch (error) { notifyError(String(error), "WORKFLOW UPDATE FAILED"); }
  };

  const handleDeleteSource = async (source: Source) => {
    if (!window.confirm(`DELETE WORKFLOW ${source.name.toUpperCase()}?`)) return;
    try {
      await api.deleteSource(source.id);
      fetchSources();
      notifySuccess(`Deleted workflow ${source.name}`);
      if (inspectedSourceId === source.id) {
        const remaining = sources.filter(s => s.id !== source.id);
        setInspectedSourceId(remaining[0]?.id ?? null);
      }
    } catch (error) {
      notifyError(String(error), "WORKFLOW DELETE FAILED");
    }
  };

  const activeRun = runningAll || !!runningSourceId || isScanning || runningSourceIds.length > 0;
  const refreshAfterRun = () => {
    fetchSources();
    for (const key of ["jobs", "job_counts", "metrics", "scheduler_status", "scrape_runs"]) void queryClient.invalidateQueries({ queryKey: [key] });
  };
  const handleRunAll = async () => {
    if (isRecordedMode() || activeRun) return;
    setRunningAll(true);
    try { const run = await api.runScrapeNow(); if (run.status === "cancelled") notifyInfo(run.error || "Committed results remain saved.", "RUN CANCELLED"); else if (["failed", "partial"].includes(run.status)) throw new Error(run.error || "Run all did not complete."); }
    catch (error) { notifyError(String(error), "RUN ALL DID NOT COMPLETE"); }
    finally { setRunningAll(false); refreshAfterRun(); }
  };
  const handleRunSourceNow = async (source: Source) => {
    if (isRecordedMode() || activeRun || source.needs_review || !workflowHasRunnableSources(source)) return;
    try {
      setRunningSourceId(source.id);
      setInspectedSourceId(source.id);
      const runPromise = api.runSourceNow(source.id);
      await refreshInspectedRunsAndLogs(source.id);
      const run = await runPromise;
      if (run.status === "cancelled") notifyInfo(run.error || "Committed results remain saved.", "RUN CANCELLED");
      else if (["failed", "partial"].includes(run.status)) throw new Error(run.error || "Workflow run did not complete.");
    } catch (e) {
      notifyError(String(e), "WORKFLOW RUN FAILED");
    } finally {
      setRunningSourceId(null);
      fetchSources();
      await refreshInspectedRunsAndLogs(source.id);
      refreshAfterRun();
    }
  };

  const handleTestSource = async (id: string) => {
    if (isRecordedMode()) return;
    try {
      setTestingSourceId(id);
      setTestEvents(null);
      const events = await api.testSource(id);
      setTestEvents(events);
    } catch (e) {
      notifyError(String(e), "WORKFLOW PREVIEW FAILED");
    } finally {
      setTestingSourceId(null);
    }
  };

  const handleSeedSampleWorkflow = async () => {
    try {
      await api.seedSampleWorkflow();
      notifySuccess(
        "Seeded Cloudflare Software Engineering Internships workflow.",
        "SAMPLE WORKFLOW SEEDED"
      );
      fetchSources();
    } catch (err) {
      console.error("Failed to seed sample workflow:", err);
    }
  };

  const handleViewInInbox = (source: Source) => {
    const company = source.name.split(" — ")[0].split(" - ")[0].trim() || source.identifier;
    setFilterSpec((prev) => ({ ...prev, search: company, statuses: ["scraped", "viewed"] }));
    setView("discovered");
  };

  // Filter sources
  const filteredSources = useMemo(() => {
    return sources.filter((s) => {
      // Tab filter
      if (activeTab === "ats") {
        if (s.kind !== "greenhouse" && s.kind !== "lever" && s.kind !== "ashby") return false;
      } else if (activeTab === "custom") {
        if (s.kind !== "selector" && s.kind !== "jsonld") return false;
      } else if (activeTab === "errors") {
        if (!sourceNeedsAttention(s)) return false;
      }

      // Search filter
      if (search.trim()) {
        const q = search.toLowerCase();
        const matchName = s.name.toLowerCase().includes(q);
        const matchId = s.identifier.toLowerCase().includes(q);
        if (!matchName && !matchId) return false;
      }

      return true;
    });
  }, [sources, activeTab, search]);

  const counts = useMemo(() => {
    const ats = sources.filter((s) => s.kind === "greenhouse" || s.kind === "lever" || s.kind === "ashby").length;
    const custom = sources.filter((s) => s.kind === "selector" || s.kind === "jsonld").length;
    const errors = sources.filter((s) => sourceNeedsAttention(s)).length;
    return { all: sources.length, ats, custom, errors };
  }, [sources]);

  const selectedRun = sourceRuns.find(run => run.id === selectedRunId) ?? null;

  const scopedLogs = useMemo(() => {
    // A workflow-owned run includes its child sources, even if they were edited later.
    if (!inspectedSource || selectedRun?.source_id === inspectedSource.id) return runLogs;
    const sourceIds = new Set([
      inspectedSource.id,
      ...sourceToWorkflowSpec(inspectedSource).sources.map(source => source.id),
    ]);
    return runLogs.filter(log => !log.source_id || sourceIds.has(log.source_id));
  }, [runLogs, inspectedSource, selectedRun]);

  // Filter logs based on level toggle / dropdown
  const displayedLogs = useMemo(() => {
    if (logFilter === "errors") {
      return scopedLogs.filter((l) => l.level === "error" || l.level === "warn");
    }
    if (logFilter === "successes") {
      return scopedLogs.filter((l) => l.level === "info");
    }
    return scopedLogs;
  }, [scopedLogs, logFilter]);

  const logCounts = useMemo(() => {
    const errors = scopedLogs.filter((l) => l.level === "error" || l.level === "warn").length;
    const successes = scopedLogs.filter((l) => l.level === "info").length;
    return { all: scopedLogs.length, successes, errors };
  }, [scopedLogs]);

  return (
    <div className="flex flex-col gap-5 w-full select-text font-mono">
      {sourceLoadError && <div role="alert" className="text-[var(--color-danger)]"><p>Could not load workflows: {sourceLoadError}</p><Button onClick={fetchSources}>Retry loading workflows</Button></div>}
      {/* Header */}
      <div className="flex flex-wrap gap-3 items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-[var(--color-ink)]">Workflows ({sources.length})</h2>
          <p className="text-xs text-[var(--color-ink-mute)]">Sources, schedules and recent results</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Tooltip content="Configure a careers source and review its matches before saving">
            <Button
              variant="inverted"
              disabled={isRecordedMode()}
              onClick={() => {
                setEditingSource(null);
                setImportedSpec(null);
                setIsAddModalOpen(true);
              }}
              aria-label="Add workflow"
            >
              ADD WORKFLOW
            </Button>
          </Tooltip>
          <Button
            variant="default"
            disabled={isRecordedMode()}
            onClick={async () => {
              try {
                const report = await api.seedAllSampleWorkflows();
                fetchSources();
                notifySuccess(`Created ${report.created.length} workflows; kept ${report.already_present.length} existing workflows. ${report.needs_review.length} need review. Scripts remain disabled until reviewed and explicitly enabled.`);
              } catch (err) {
                notifyError(`Failed to load test workflows: ${err}`);
              }
            }}
            aria-label="Load test workflows"
          >
            Load test workflows
          </Button>
          {activeRun ? <>
            <span role="status" className="max-w-48 truncate text-xs">{scanStatusText || "Running workflows…"}</span>
            <Button variant="danger" onClick={() => api.cancelScrape().then(cancelled => notifyInfo(cancelled ? "Cancellation requested; committed results remain saved." : "No active run to cancel.")).catch(error => notifyError(String(error)))}>Cancel run</Button>
          </> : <Button variant="default" disabled={isRecordedMode() || !sources.some(source => workflowHasRunnableSources(source) && !source.needs_review)} onClick={() => void handleRunAll()}>Run all</Button>}
          <WorkflowImportButton disabled={isRecordedMode() || isAddModalOpen} onImport={spec => { setEditingSource(null); setImportedSpec(spec); setIsAddModalOpen(true); }} />
        </div>
      </div>

      {isRecordedMode() && <p className="text-sm text-[var(--color-ink-mute)]">Recorded examples cannot run, test or import workflows. Open the desktop application to use live sources.</p>}
      <div className="flex flex-col lg:flex-row gap-5 items-start w-full min-h-0">
        {/* Left Column: Workflow Index */}
        <div className="w-full lg:w-[450px] shrink-0 flex flex-col gap-3">
          {/* Sub-Filter Segmented Row + Search */}
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap gap-1">
              <button
                onClick={() => setActiveTab("all")}
                className={`px-2.5 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
                  activeTab === "all"
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                ALL ({counts.all})
              </button>
              <button
                onClick={() => setActiveTab("ats")}
                className={`px-2.5 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
                  activeTab === "ats"
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                ATS ({counts.ats})
              </button>
              <button
                onClick={() => setActiveTab("custom")}
                className={`px-2.5 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
                  activeTab === "custom"
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                CUSTOM ({counts.custom})
              </button>
              <button
                onClick={() => setActiveTab("errors")}
                className={`px-2.5 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
                  activeTab === "errors"
                    ? "bg-[var(--color-danger)] text-[var(--color-paper)] border-[var(--color-danger)]"
                    : "bg-[var(--color-paper)] text-[var(--color-danger)] border-[var(--color-danger)] hover:bg-[var(--color-cherry)]/10"
                }`}
              >
                ERRORS ({counts.errors})
              </button>
            </div>
            <div>
              <Input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="SEARCH WORKFLOWS..."
                className="text-xs"
              />
            </div>
          </div>

          {/* Workflows Table */}
          {sources.length === 0 ? (
            <div className="p-8 border border-dashed border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 flex flex-col items-center justify-center text-center gap-3">
              <span className="text-2xl text-[var(--color-ink)]">■</span>
              <div className="flex flex-col gap-1 max-w-md">
                <span className="font-bold text-sm uppercase tracking-wider text-[var(--color-ink)]">
                  NO WORKFLOWS IN REGISTRY
                </span>
                <span className="text-xs text-[var(--color-ink-mute)]">
                  Add a careers URL or import workflow YAML to begin. You can also load the sample Cloudflare configuration.
                </span>
              </div>
              <Button
                variant="inverted"
                onClick={handleSeedSampleWorkflow}
              >
                LOAD SAMPLE SWE INTERNSHIP WORKFLOW
              </Button>
            </div>
          ) : filteredSources.length === 0 ? (
            <div className="p-8 border border-dashed border-[var(--color-ink-mute)] text-center text-xs font-mono text-[var(--color-ink-mute)]">
              NO MATCHING WORKFLOWS FOUND. CLICK &quot;+ ADD WORKFLOW&quot; TO CONFIGURE A TARGET.
            </div>
          ) : (
            <div className="border border-[var(--color-border)] bg-[var(--color-surface)]">
              <Table className="w-full">
                <TableHead>
                  <TableRow className="border-b border-[var(--color-border)] bg-[var(--color-surface-raised)]/50 text-[10px]">
                    <TableHeader className="w-10 py-1.5 px-2">ON</TableHeader>
                    <TableHeader className="py-1.5 px-2">WORKFLOW</TableHeader>
                    <TableHeader className="w-24 py-1.5 px-1 text-right">STATUS</TableHeader>
                    <TableHeader className="w-24 py-1.5 px-2 text-right">ACTIONS</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {filteredSources.map((s) => {
                    const isRunning = runningSourceId === s.id || runningSourceIds.includes(s.id);
                    const isTesting = testingSourceId === s.id;
                    const isSelected = inspectedSource?.id === s.id;
                    const statusVisual = s.needs_review
                      ? { label: "NEEDS REVIEW", colorClass: "text-[var(--color-warning)]", bulletClass: "text-[var(--color-warning)]", bulletGlyph: "●" }
                      : getWorkflowStatus(s, isRunning);

                    return (
                      <TableRow
                        key={s.id}
                        data-source-id={s.id}
                        onClick={() => setInspectedSourceId(s.id)}
                        className={`cursor-pointer transition-colors border-b border-[var(--color-border)] last:border-b-0 ${
                          isSelected
                            ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-semibold"
                            : "hover:bg-[var(--color-surface-raised)] text-[var(--color-ink)]"
                        }`}
                      >
                        <TableCell className="w-10 py-2 px-2" onClick={(e) => e.stopPropagation()}>
                          <Toggle
                            checked={s.enabled}
                            disabled={s.needs_review}
                            onChange={(checked) => handleToggleSource(s, checked)}
                          />
                        </TableCell>
                        <TableCell className="py-2 px-2 min-w-0">
                          <div className="flex flex-col items-start min-w-0">
                            <button
                              type="button"
                              aria-label={`Edit workflow ${s.name}`}
                              onClick={(e) => {
                                e.stopPropagation();
                                setImportedSpec(null);
                                setEditingSource(s);
                                setIsAddModalOpen(true);
                              }}
                              className={`font-bold tracking-tight text-left truncate max-w-full hover:underline focus-visible:outline focus-visible:outline-2 ${
                                isSelected ? "text-[var(--color-paper)] focus-visible:outline-[var(--color-paper)]" : "text-[var(--color-ink)] focus-visible:outline-[var(--color-signal)]"
                              }`}
                            >
                              {s.name}
                            </button>
                            <span className={`text-[11px] truncate max-w-full font-mono ${isSelected ? "text-[var(--color-paper)]/75" : "text-[var(--color-ink-mute)]"}`}>
                              {s.identifier}
                            </span>
                          </div>
                        </TableCell>
                        <TableCell className="w-24 py-2 px-1 text-right whitespace-nowrap">
                          <div className="flex items-center justify-end gap-1" data-testid={`status-${s.id}`}>
                            <span className={`text-xs ${statusVisual.bulletClass}`}>
                              {statusVisual.bulletGlyph}
                            </span>
                            {sourceNeedsAttention(s) && !s.needs_review ? (
                              <button
                                type="button"
                                 aria-label={`View failed run for ${s.name}`}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setInspectedSourceId(s.id);
                                }}
                                className={`text-[10px] font-mono uppercase font-bold underline-offset-2 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-signal)] ${
                                  isSelected ? "text-[var(--color-paper)]" : statusVisual.colorClass
                                }`}
                              >
                                {statusVisual.label}
                              </button>
                            ) : (
                              <span className={`text-[10px] font-mono uppercase font-bold ${
                                isSelected ? "text-[var(--color-paper)]" : statusVisual.colorClass
                              }`}>
                                {statusVisual.label}
                              </span>
                            )}
                          </div>
                        </TableCell>
                        <TableCell className="w-24 py-2 px-2 text-right whitespace-nowrap" onClick={(e) => e.stopPropagation()}>
                          <div className="flex items-center justify-end gap-1">
                            <Tooltip content={workflowHasRunnableSources(s) ? "Run this workflow immediately into database" : "Workflow or all child sources are disabled. Open workflow editor to enable them before running."}>
                              <button
                                onClick={() => handleRunSourceNow(s)}
                                disabled={isRecordedMode() || activeRun || s.needs_review || !workflowHasRunnableSources(s)}
                                className={`px-2 py-0.5 border text-xs font-mono uppercase cursor-pointer disabled:opacity-50 ${
                                  isSelected
                                    ? "border-[var(--color-paper)] bg-[var(--color-paper)] text-[var(--color-ink)] hover:bg-[var(--color-surface-raised)]"
                                    : "border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-80"
                                }`}
                              >
                                {isRunning ? "RUNNING…" : "RUN"}
                              </button>
                            </Tooltip>
                            <ActionMenu label={`More actions for ${s.name}`} items={[
                              { label: "Edit workflow", onSelect: () => { setImportedSpec(null); setEditingSource(s); setIsAddModalOpen(true); } },
                              { label: isTesting ? "Previewing matches…" : "Preview matches", disabled: isRecordedMode() || isTesting || activeRun || s.needs_review, onSelect: () => void handleTestSource(s.id) },
                              { label: "View run history", onSelect: () => setInspectedSourceId(s.id) },
                              { label: "Delete workflow", danger: true, dividerBefore: true, onSelect: () => void handleDeleteSource(s) },
                            ]} />
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}
        </div>

        {/* Right Column: Workflow Dossier & Execution Observability */}
        <div className="flex-1 min-w-0 w-full">
          {inspectedSource ? (
            <div className="flex flex-col gap-4 p-5 border border-[var(--color-border)] bg-[var(--color-surface)]">
              {/* Dossier Header */}
              <div className="flex flex-col gap-3 pb-3 border-b border-[var(--color-border)]">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-bold text-[var(--color-text-muted)] uppercase tracking-wider">
                      WORKFLOW DOSSIER //
                    </span>
                    <span className="px-1.5 py-0.2 border border-[var(--color-border)] text-[10px] uppercase font-mono">
                      {inspectedSource.kind}
                    </span>
                    <span className="px-1.5 py-0.2 border border-[var(--color-border)] text-[10px] uppercase font-mono">
                      {inspectedSource.cadence}
                    </span>
                  </div>
                  <IconButton label={`Close workflow logs for ${inspectedSource.name}`} onClick={() => setInspectedSourceId(null)} variant="danger">
                    <Icon name="close" />
                  </IconButton>
                </div>

                <div>
                  <h3 className="text-2xl font-bold tracking-tight uppercase text-[var(--color-ink)]">
                    {inspectedSource.name}
                  </h3>
                  <p className="text-xs font-mono text-[var(--color-ink-mute)] break-all mt-0.5">
                    {inspectedSource.identifier}
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-2 pt-1">
                  <Button
                    variant="inverted"
                    disabled={isRecordedMode() || activeRun || inspectedSource.needs_review || !workflowHasRunnableSources(inspectedSource)}
                    onClick={() => handleRunSourceNow(inspectedSource)}
                    title={workflowHasRunnableSources(inspectedSource) ? undefined : "Workflow or all child sources are disabled. Open workflow editor to enable them before running."}
                  >
                    {runningSourceId === inspectedSource.id ? "RUNNING WORKFLOW…" : "RUN WORKFLOW NOW"}
                  </Button>
                  <Button
                    disabled={isRecordedMode() || testingSourceId === inspectedSource.id || activeRun || inspectedSource.needs_review}
                    onClick={() => void handleTestSource(inspectedSource.id)}
                  >
                    {testingSourceId === inspectedSource.id ? "PREVIEWING MATCHES…" : "PREVIEW MATCHES"}
                  </Button>
                  <Button
                    onClick={() => {
                      setImportedSpec(null);
                      setEditingSource(inspectedSource);
                      setIsAddModalOpen(true);
                    }}
                  >
                    EDIT WORKFLOW
                  </Button>
                  <Button
                    onClick={() => handleViewInInbox(inspectedSource)}
                  >
                    <span>VIEW JOBS IN INBOX</span><Icon name="right" />
                  </Button>
                </div>
              </div>

              {!workflowHasRunnableSources(inspectedSource) && (
                <div role="alert" className="border border-[var(--color-warning)] p-2 text-xs text-[var(--color-warning)]">
                  Workflow or all sources disabled. Enable the workflow and at least one source before running.{" "}
                  <button type="button" className="underline" onClick={() => { setImportedSpec(null); setEditingSource(inspectedSource); setIsAddModalOpen(true); }}>Edit workflow</button>
                </div>
              )}

              {/* Telemetry Row */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 p-3 border border-[var(--color-border)] bg-[var(--color-surface-raised)]/40 text-xs">
                <div>
                  <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-semibold">TARGET IDENTIFIER:</span>
                  <div className="font-mono text-xs truncate text-[var(--color-ink)] mt-0.5">{inspectedSource.identifier}</div>
                </div>
                <div>
                  <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-semibold">CADENCE / SCHEDULE:</span>
                  <div className="font-mono text-xs text-[var(--color-ink)] mt-0.5">
                    {inspectedSource.cadence === "manual" ? "MANUAL ONLY" : `${inspectedSource.cadence.toUpperCase()} · ${workflowTimezone(inspectedSource, settings)}`}
                    {inspectedSource.next_run_at ? ` (NEXT: ${formatDateTime(inspectedSource.next_run_at, settings)})` : ""}
                  </div>
                </div>
                <div>
                  <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-semibold">LAST DURATION:</span>
                  <div className="font-mono text-xs text-[var(--color-ink)] mt-0.5">
                    {inspectedSource.last_run_duration_ms
                      ? `${(Number(inspectedSource.last_run_duration_ms) / 1000).toFixed(2)}s`
                      : "NONE"}
                  </div>
                </div>
                <div>
                  <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-semibold">TTL RETENTION:</span>
                  <div className="font-mono text-xs text-[var(--color-ink)] mt-0.5">
                    {Number(inspectedSource.retention_days) === 0
                      ? "KEEP FOREVER"
                      : `${inspectedSource.retention_days} DAYS (${inspectedSource.retention_policy})`}
                  </div>
                </div>
              </div>

              {/* Observability Section Header */}
              <div className="flex items-center justify-between border-b border-[var(--color-border)] pb-2 font-bold">
                <span className="text-xs font-bold uppercase tracking-wider text-[var(--color-text)]">
                  {inspectedSource.name} // WORKFLOW OBSERVABILITY & EXECUTION LOGS
                </span>
              </div>

              {/* Master-Detail Split: Runs History (Left) + Terminal Log Stream (Right) */}
              <div className="grid grid-cols-12 gap-3 pt-1">
                {/* Left 4 Cols: Recent Runs List */}
                <div className="col-span-12 md:col-span-4 flex flex-col gap-1.5 border border-[var(--color-border)] bg-[var(--color-surface)] p-2">
                  <span className="font-bold text-xs uppercase tracking-wider text-[var(--color-ink)] border-b border-[var(--color-border)] pb-1">
                    EXECUTION RUNS (LAST 10)
                  </span>

                  {isLoadingRuns ? (
                    <span className="text-[var(--color-ink-mute)] text-xs p-2">LOADING RUNS…</span>
                  ) : sourceRuns.length === 0 ? (
                    <span className="text-[var(--color-ink-mute)] text-xs p-2">
                      NO ISOLATED RUNS RECORDED YET. CLICK &quot;RUN WORKFLOW NOW&quot; TO TRIGGER.
                    </span>
                  ) : (
                    <div className="flex flex-col divide-y divide-[var(--color-border)] max-h-72 overflow-y-auto">
                      {sourceRuns.map((r) => {
                        const isSelected = selectedRunId === r.id;
                        return (
                          <div
                            key={r.id}
                            onClick={() => setSelectedRunId(r.id)}
                            className={`p-2 flex flex-col gap-0.5 cursor-pointer text-xs transition-none ${
                              isSelected
                                ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                                : "hover:bg-[var(--color-surface-raised)]"
                            }`}
                          >
                            <div className="flex flex-wrap gap-2 items-center justify-between">
                              <div className="flex items-center gap-1.5">
                                <span
                                  className={`px-1 py-0.2 text-[9px] uppercase font-bold border ${
                                    runStatusBadge(r.status)
                                  }`}
                                >
                                  {runStatusLabel(r.status)}
                                </span>
                                <span className="text-xs">
                                  {formatDateTime(r.started_at, settings)}
                                </span>
                              </div>
                              <span className="text-xs">
                                {r.status === "running" ? "RUNNING…" : `+${Number(r.jobs_new)}`}
                              </span>
                            </div>
                            {r.counts && <p className="text-xs mt-1">Examined {r.counts.examination_incomplete ? "at least " : ""}{r.counts.fetched} · rejected {r.counts.examination_incomplete ? "unknown" : r.counts.rejected} · blocked {r.counts.examination_incomplete ? "at least " : ""}{r.counts.blocked} · inserted {r.counts.inserted} · updated {r.counts.updated} · failed {r.counts.failed} · enrichment warnings {r.counts.enrichment_warnings ?? 0} · retention removed {r.counts.retention_removed}</p>}
                            {r.counts?.examination_incomplete && <p className="text-xs">Worker counters are incomplete; saved-record counts are confirmed.</p>}
                            {r.error && (
                              <span className={`text-xs truncate ${isSelected ? "text-[var(--color-paper)]" : r.status === "success_with_warnings" ? "text-[var(--color-warning)]" : r.status === "cancelled" ? "text-[var(--color-ink-mute)]" : "text-[var(--color-danger)]"}`}>
                                {r.status === "success_with_warnings" ? "WARNING" : r.status === "cancelled" ? "CANCELLED" : "DETAIL"}: {r.error}
                              </span>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* Right 8 Cols: Terminal Execution Log Viewer */}
                <div className="col-span-12 md:col-span-8 flex flex-col gap-1.5 border border-[var(--color-border)] bg-[var(--color-surface)] p-2">
                  <div className="flex items-center justify-between border-b border-[var(--color-border)] pb-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-bold text-xs uppercase tracking-wider text-[var(--color-ink)]">
                        TERMINAL LOG STREAM {selectedRun ? `· RUN ${selectedRun.id.slice(0, 8)}` : ""}
                      </span>
                      {selectedRun?.status === "running" && (
                        <span className="px-1.5 py-0.2 text-[9px] bg-[var(--color-surface-raised)] text-[var(--color-ink)] border border-[var(--color-border)] animate-pulse font-mono font-bold">
                          EXECUTING…
                        </span>
                      )}
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-xs text-[var(--color-ink-mute)] font-mono uppercase">
                        FILTER:
                      </span>
                      <Select aria-label="Filter execution logs" value={logFilter}
                        onChange={value => setLogFilter(value as "all" | "successes" | "errors")}
                        options={[
                          { value: "all", label: `ALL LOGS (${logCounts.all})` },
                          { value: "successes", label: `SUCCESSES (${logCounts.successes})` },
                          { value: "errors", label: `ERRORS (${logCounts.errors})` },
                        ]} />
                    </div>
                  </div>

                  <ScanDiagnostics sources={selectedRun?.counts?.sources} />
                  {isLoadingLogs ? (
                    <div className="p-4 text-center text-xs text-[var(--color-ink-mute)] font-mono">
                      STREAMING LOGS…
                    </div>
                  ) : !selectedRunId ? (
                    <div className="p-4 text-center text-xs text-[var(--color-ink-mute)] font-mono">
                      SELECT AN EXECUTION RUN ON THE LEFT TO INSPECT ITS TERMINAL LOGS.
                    </div>
                  ) : displayedLogs.length === 0 ? (
                    <div className="p-4 text-center text-xs text-[var(--color-ink-mute)] font-mono">
                      NO LOG EVENTS RECORDED FOR THIS RUN.
                    </div>
                  ) : (
                    <div className="flex flex-col gap-1 max-h-72 overflow-y-auto font-mono text-xs p-1 bg-[var(--color-surface-raised)]/20 border border-[var(--color-border)]">
                      {displayedLogs.map((log) => {
                        const isError = log.level === "error";
                        const isWarn = log.level === "warn";
                        return (
                          <div
                            key={log.id}
                            className={`p-1.5 border text-xs leading-relaxed ${
                              isError
                                ? "border-[var(--color-danger)] bg-[var(--color-surface-raised)] text-[var(--color-danger)]"
                                : isWarn
                                ? "border-[var(--color-warning)] bg-[var(--color-surface-raised)] text-[var(--color-warning)]"
                                : "border-[var(--color-border)]/40 bg-[var(--color-surface)] text-[var(--color-ink)]"
                            }`}
                          >
                            <div className="flex items-center justify-between text-xs opacity-80 mb-0.5 font-bold">
                              <span className={isError ? "text-[var(--color-danger)]" : isWarn ? "text-[var(--color-warning)]" : "text-[var(--color-ink)]"}>
                                [{log.level.toUpperCase()}]
                              </span>
                              <span>
                                {formatTime(log.timestamp, settings)}
                              </span>
                            </div>
                            <div className="break-all whitespace-pre-wrap">
                              {log.message}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center p-12 text-center border border-dashed border-[var(--color-border)] bg-[var(--color-surface)] h-full min-h-[360px] gap-3">
              <span className="text-3xl text-[var(--color-text-muted)]">■</span>
              <h4 className="text-sm font-bold uppercase tracking-wider text-[var(--color-text)]">
                SELECT A WORKFLOW
              </h4>
              <p className="text-xs text-[var(--color-text-muted)] max-w-sm">
                Choose a workflow from the index on the left to inspect its criteria, preview matches, and view observability logs.
              </p>
              {filteredSources.length > 0 && (
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => setInspectedSourceId(filteredSources[0].id)}
                >
                  INSPECT {filteredSources[0].name.toUpperCase()}
                </Button>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Dry Run Preview Drawer */}
      {testEvents && (
        <div className="flex flex-col gap-2 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 font-mono text-xs select-text">
          <div className="flex justify-between items-center font-bold">
            <span>DRY-RUN EXTRACTION PREVIEW (FIRST 10 JOBS)</span>
            <button
              onClick={() => setTestEvents(null)}
              className="text-xs hover:opacity-75 cursor-pointer"
            >
              [CLOSE]
            </button>
          </div>
          <div className="max-h-48 overflow-y-auto flex flex-col gap-1 border border-[var(--color-ink)] bg-[var(--color-paper)] p-2">
            {testEvents.length === 0 ? (
              <span className="text-[var(--color-ink-mute)]">
                NO JOBS MATCHED BY ADAPTER.
              </span>
            ) : (
              testEvents.map((ev, i) => (
                <div key={i} className="text-xs truncate">
                  {ev.type === "job" ? (
                    <span>
                      <strong className="text-[var(--color-ink)]">{ev.title}</strong> · {ev.company} · {ev.location || "Location unknown"} ({ev.role_type})
                      {ev.salary?.raw ? ` · ${ev.salary.raw}` : ""}
                    </span>
                  ) : ev.type === "warning" ? (
                    <span className="text-[var(--color-warning)]">{ev.eligibility_incomplete ? "INCOMPLETE ELIGIBILITY" : "WARNING"}: {ev.message}</span>
                  ) : ev.type === "error" ? (
                    <span className="text-[var(--color-danger)]">
                      ERROR: {ev.message}
                    </span>
                  ) : (
                    <span className="text-[var(--color-ink-mute)]">
                      PROGRESS: {ev.type}
                    </span>
                  )}
                </div>
              ))
            )}
          </div>
          <ScanDiagnostics sources={testEvents.find(event => event.type === "done")?.stats.sources} />
        </div>
      )}

      <AddSourceModal
        isOpen={isAddModalOpen}
        onClose={() => {
          setIsAddModalOpen(false);
          setEditingSource(null);
          setImportedSpec(null);
        }}
        onSourceSaved={fetchSources}
        initialSource={editingSource}
        initialSpec={importedSpec}
      />
    </div>
  );
}
