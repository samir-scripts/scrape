import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import { WorkflowImportButton } from "./WorkflowImportButton";
import { IconButton } from "../../ui/IconButton";
import { Icon } from "../../ui/Icon";
import { isRecordedMode } from "../../../lib/runtime";
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
import { Input } from "../../ui/Input";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { Tooltip } from "../../ui/Tooltip";
import { Select } from "../../ui/Select";
import { AddSourceModal } from "./AddSourceModal";
import { notifyError, notifySuccess, notifyInfo } from "../../../lib/toast";
import { getWorkflowStatus } from "../../../lib/status";

type FilterTab = "all" | "ats" | "custom" | "errors";

interface SourcesSettingsProps {
  settings?: Settings;
  onUpdateSetting?: (key: string, value: unknown) => void;
}

export function SourcesSettings({ settings, onUpdateSetting }: SourcesSettingsProps = {}) {
  const queryClient = useQueryClient();
  const [sources, setSources] = useState<Source[]>([]);
  const [sourceLoadError, setSourceLoadError] = useState("");
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<FilterTab>("all");

  // Modals & drawers
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [importedSpec, setImportedSpec] = useState<WorkflowSpec | null>(null);
  const [runningAll, setRunningAll] = useState(false);
  const [showMaintenance, setShowMaintenance] = useState(false);
  const [editingSource, setEditingSource] = useState<Source | null>(null);

  // Per-job execution state
  const [runningSourceId, setRunningSourceId] = useState<string | null>(null);
  const [testingSourceId, setTestingSourceId] = useState<string | null>(null);
  const [testEvents, setTestEvents] = useState<WorkerEvent[] | null>(null);

  // Inspector & Consolidated Logs Dock state
  const [inspectedSource, setInspectedSource] = useState<Source | null>(null);
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
      if (inspectedSource?.id === source.id) setInspectedSource(null);
      fetchSources();
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
    try { const run = await api.runScrapeNow(); if (["failed", "partial"].includes(run.status)) throw new Error(run.error || "Run all did not complete."); }
    catch (error) { notifyError(String(error), "RUN ALL DID NOT COMPLETE"); }
    finally { setRunningAll(false); refreshAfterRun(); }
  };
  const handleRunSourceNow = async (source: Source) => {
    if (isRecordedMode() || activeRun) return;
    try {
      setRunningSourceId(source.id);
      setInspectedSource(source);
      const runPromise = api.runSourceNow(source.id);
      await refreshInspectedRunsAndLogs(source.id);
      const run = await runPromise;
      if (["failed", "partial"].includes(run.status)) throw new Error(run.error || "Workflow run did not complete.");
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

  const handlePruneExpired = async () => {
    try {
      const pruned = await api.cleanupExpiredJobs();
      notifySuccess(`Pruned ${pruned} expired postings according to TTL policies.`, "TTL PRUNE COMPLETE");
      fetchSources();
    } catch (error) {
      notifyError(String(error), "RETENTION CLEANUP FAILED");
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
        if (s.last_status !== "error") return false;
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
    const errors = sources.filter((s) => s.last_status === "error").length;
    return { all: sources.length, ats, custom, errors };
  }, [sources]);

  // Filter logs based on level toggle / dropdown
  const displayedLogs = useMemo(() => {
    const scopedLogs = inspectedSource
      ? runLogs.filter((l) => !l.source_id || l.source_id === inspectedSource.id)
      : runLogs;

    if (logFilter === "errors") {
      return scopedLogs.filter((l) => l.level === "error" || l.level === "warn");
    }
    if (logFilter === "successes") {
      return scopedLogs.filter((l) => l.level === "info");
    }
    return scopedLogs;
  }, [runLogs, logFilter, inspectedSource]);

  const logCounts = useMemo(() => {
    const scopedLogs = inspectedSource
      ? runLogs.filter((l) => !l.source_id || l.source_id === inspectedSource.id)
      : runLogs;
    const errors = scopedLogs.filter((l) => l.level === "error" || l.level === "warn").length;
    const successes = scopedLogs.filter((l) => l.level === "info").length;
    return { all: scopedLogs.length, successes, errors };
  }, [runLogs, inspectedSource]);

  const selectedRun = useMemo(() => {
    return sourceRuns.find((r) => r.id === selectedRunId) || null;
  }, [sourceRuns, selectedRunId]);

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
          {settings && onUpdateSetting && <Toggle label="Automatic scraping enabled" checked={settings.scraper_enabled} onChange={(enabled) => onUpdateSetting("scraper.enabled", enabled)} />}
          <WorkflowImportButton disabled={isRecordedMode() || isAddModalOpen} onImport={spec => { setEditingSource(null); setImportedSpec(spec); setIsAddModalOpen(true); }} />
          <Button variant="default" disabled={isRecordedMode() || activeRun || !sources.some(source => source.enabled && !source.needs_review)} onClick={() => void handleRunAll()}>Run all</Button>
          {activeRun && <Button variant="danger" onClick={() => api.cancelScrape().then(cancelled => notifyInfo(cancelled ? "Cancellation requested; committed results remain saved." : "No active run to cancel.")).catch(error => notifyError(String(error)))}>Cancel active run</Button>}
          <IconButton label="Workflow maintenance" onClick={() => setShowMaintenance(value => !value)}><Icon name="settings" /></IconButton>
          <Tooltip content="Configure a careers source and review its matches before saving">
            <Button
              variant="inverted"
              disabled={isRecordedMode()}
              onClick={() => {
                setEditingSource(null);
                setImportedSpec(null);
                setIsAddModalOpen(true);
              }}
              shortcut="+ ADD"
            >
              ADD WORKFLOW
            </Button>
          </Tooltip>
        </div>
      </div>

      {isRecordedMode() && <p className="text-sm text-[var(--color-ink-mute)]">Recorded examples cannot run, test or import workflows. Open the desktop application to use live sources.</p>}
      {activeRun && <p role="status" className="text-sm">{scanStatusText || "Running workflows…"}</p>}
      {showMaintenance && <div className="border p-3"><Button onClick={handlePruneExpired}>Cleanup expired discoveries</Button><p className="text-xs">Tracked jobs, notes and favorites are protected.</p></div>}
      {/* Sub-Filter Segmented Row + Search */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap gap-1">
          <button
            onClick={() => setActiveTab("all")}
            className={`px-3 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
              activeTab === "all"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
            }`}
          >
            ALL ({counts.all})
          </button>
          <button
            onClick={() => setActiveTab("ats")}
            className={`px-3 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
              activeTab === "ats"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
            }`}
          >
            ATS ({counts.ats})
          </button>
          <button
            onClick={() => setActiveTab("custom")}
            className={`px-3 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
              activeTab === "custom"
                ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
            }`}
          >
            CUSTOM ({counts.custom})
          </button>
          <button
            onClick={() => setActiveTab("errors")}
            className={`px-3 py-1 text-xs uppercase tracking-wider font-bold cursor-pointer border ${
              activeTab === "errors"
                ? "bg-[var(--color-danger)] text-[var(--color-paper)] border-[var(--color-danger)]"
                : "bg-[var(--color-paper)] text-[var(--color-danger)] border-[var(--color-danger)] hover:bg-[var(--color-cherry)]/10"
            }`}
          >
            ERRORS ({counts.errors})
          </button>
        </div>

        <div className="w-64">
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="SEARCH WORKFLOWS..."
            className="text-xs"
          />
        </div>
      </div>

      <Rule />

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
        <Table>
          <TableHead>
            <TableRow>
              <TableHeader>
                <Tooltip content="Toggle automated scheduling for this workflow">
                  <span>ON</span>
                </Tooltip>
              </TableHeader>
              <TableHeader>WORKFLOW / COMPANY</TableHeader>
              <TableHeader>
                <Tooltip content="Target extraction adapter: Greenhouse, Lever, Ashby, JSON-LD, or Custom Selector">
                  <span>EXTRACTOR</span>
                </Tooltip>
              </TableHeader>
              <TableHeader>
                <Tooltip content="Dedicated execution frequency for this workflow">
                  <span>CADENCE</span>
                </Tooltip>
              </TableHeader>
              <TableHeader>
                <Tooltip content="Data retention lifespan and untracked pruning policy">
                  <span>RETENTION</span>
                </Tooltip>
              </TableHeader>
              <TableHeader>
                <Tooltip content="Status of the most recent execution run">
                  <span>STATUS</span>
                </Tooltip>
              </TableHeader>
              <TableHeader>ACTIONS</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {filteredSources.map((s) => {
              const isRunning = runningSourceId === s.id || runningSourceIds.includes(s.id);
              const isTesting = testingSourceId === s.id;
              const isInspected = inspectedSource?.id === s.id;
              const statusVisual = s.needs_review
                ? { label: "NEEDS REVIEW", colorClass: "text-[var(--color-warning)]", bulletClass: "text-[var(--color-warning)]", bulletGlyph: "●" }
                : getWorkflowStatus(s, isRunning);

              return (
                <TableRow
                  key={s.id}
                  className={isInspected ? "bg-[var(--color-paper-2)]/60 font-semibold" : ""}
                >
                  <TableCell>
                    <Toggle
                      checked={s.enabled}
                      disabled={s.needs_review}
                      onChange={(checked) => handleToggleSource(s, checked)}
                    />
                  </TableCell>
                  <TableCell>
                    <div
                      onClick={() => setInspectedSource(isInspected ? null : s)}
                      className="flex flex-col cursor-pointer group"
                    >
                      <span className="font-bold tracking-tight text-[var(--color-ink)] group-hover:underline">
                        {s.name}
                      </span>
                      <span className="text-xs text-[var(--color-ink-mute)] truncate max-w-[240px]">
                        {s.identifier}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="px-1.5 py-0.5 text-xs font-mono uppercase border border-[var(--color-ink)] bg-[var(--color-paper-2)]">
                      {s.kind}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="px-1.5 py-0.5 text-xs font-mono uppercase font-bold border border-[var(--color-ink)]">
                      {s.cadence === "manual" ? "Manual" : s.cadence.replace("_", " ")}
                    </span>
                    {s.next_run_at && <p className="text-xs mt-1">Next: {formatDateTime(s.next_run_at, settings)}</p>}
                  </TableCell>
                  <TableCell className="text-xs font-mono whitespace-nowrap">
                    {Number(s.retention_days) === 0
                      ? "FOREVER"
                      : `${s.retention_days}D (${s.retention_policy === "untracked_only" ? "SAFE" : "ALL"})`}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5" data-testid={`status-${s.id}`}>
                      <span className={`text-xs ${statusVisual.bulletClass}`}>
                        {statusVisual.bulletGlyph}
                      </span>
                      <span className={`text-xs font-mono uppercase font-bold ${statusVisual.colorClass}`}>
                        {statusVisual.label}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5">
                      <Tooltip content="Run this workflow immediately into database">
                        <button
                          onClick={() => handleRunSourceNow(s)}
                          disabled={isRecordedMode() || activeRun || s.needs_review}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-80 text-xs font-mono uppercase cursor-pointer disabled:opacity-50"
                        >
                          {isRunning ? "RUNNING…" : "RUN"}
                        </button>
                      </Tooltip>

                      <Tooltip content="Dry run extraction without writing to database">
                        <button
                          onClick={() => handleTestSource(s.id)}
                          disabled={isRecordedMode() || isTesting || activeRun || s.needs_review}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-xs font-mono uppercase cursor-pointer"
                        >
                          {isTesting ? "…" : "TEST"}
                        </button>
                      </Tooltip>

                      <Tooltip content="View run history, duration telemetry, and live execution logs">
                        <button
                          onClick={() => setInspectedSource(isInspected ? null : s)}
                          className={`px-2 py-0.5 border text-xs font-mono uppercase cursor-pointer ${
                            isInspected
                              ? "border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                              : "border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)]"
                          }`}
                        >
                          {isInspected ? "HIDE" : "LOGS"}
                        </button>
                      </Tooltip>

                      <Tooltip content="Edit workflow target, cadence, or filters">
                        <button
                          onClick={() => {
                            setImportedSpec(null); setEditingSource(s);
                            setIsAddModalOpen(true);
                          }}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-xs font-mono uppercase cursor-pointer"
                        >
                          EDIT
                        </button>
                      </Tooltip>

                      <Tooltip content={`Delete workflow ${s.name}`}>
                        <IconButton
                          label={`Delete workflow ${s.name}`}
                          variant="danger"
                          onClick={() => void handleDeleteSource(s)}
                          className="min-h-8 min-w-8"
                        >
                          <Icon name="trash" />
                        </IconButton>
                      </Tooltip>
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}

      {/* Consolidated Workflow Inspector & Execution Logs Dock */}
      {inspectedSource && (
        <div className="flex flex-col gap-3 p-4 border-2 border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-xs">
          {/* Dock Header */}
          <div className="flex justify-between items-center border-b border-[var(--color-ink)] pb-2 font-bold">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm uppercase tracking-tight text-[var(--color-ink)]">
                {inspectedSource.name} // WORKFLOW OBSERVABILITY & EXECUTION LOGS
              </span>
              <span className="px-1.5 py-0.5 text-xs border border-[var(--color-ink)] bg-[var(--color-paper)] uppercase">
                {inspectedSource.kind}
              </span>
              <span className="px-1.5 py-0.5 text-xs border border-[var(--color-ink)] bg-[var(--color-paper)] uppercase">
                {inspectedSource.cadence}
              </span>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <button
                onClick={() => handleRunSourceNow(inspectedSource)}
                disabled={isRecordedMode() || activeRun}
                className="px-2.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-85 text-xs uppercase font-bold cursor-pointer disabled:opacity-50"
              >
                {runningSourceId === inspectedSource.id ? "RUNNING WORKFLOW…" : "RUN WORKFLOW NOW"}
              </button>
              <button
                onClick={() => handleViewInInbox(inspectedSource)}
                className="px-2.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-xs uppercase font-bold cursor-pointer inline-flex items-center gap-2"
              >
                <span>VIEW JOBS IN INBOX</span><Icon name="right" />
              </button>
              <IconButton label={`Close workflow logs for ${inspectedSource.name}`} onClick={() => setInspectedSource(null)} variant="danger">
                <Icon name="close" />
              </IconButton>
            </div>
          </div>

          {/* Telemetry Row */}
          <div className="grid grid-cols-4 gap-2 p-2 border border-[var(--color-ink)] bg-[var(--color-paper)] text-xs">
            <div>
              <span className="text-xs text-[var(--color-ink-mute)] uppercase">TARGET URL:</span>
              <div className="font-mono truncate">{inspectedSource.identifier}</div>
            </div>
            <div>
              <span className="text-xs text-[var(--color-ink-mute)] uppercase">CADENCE / NEXT RUN:</span>
              <div className="font-mono">
                {inspectedSource.cadence.toUpperCase()}
                {inspectedSource.next_run_at ? ` · ${inspectedSource.next_run_at.slice(0, 16)}` : ""}
              </div>
            </div>
            <div>
              <span className="text-xs text-[var(--color-ink-mute)] uppercase">LAST DURATION:</span>
              <div className="font-mono">
                {inspectedSource.last_run_duration_ms
                  ? `${(Number(inspectedSource.last_run_duration_ms) / 1000).toFixed(2)}s`
                  : "NONE"}
              </div>
            </div>
            <div>
              <span className="text-xs text-[var(--color-ink-mute)] uppercase">TTL RETENTION:</span>
              <div className="font-mono">
                {Number(inspectedSource.retention_days) === 0
                  ? "KEEP FOREVER"
                  : `${inspectedSource.retention_days} DAYS (${inspectedSource.retention_policy})`}
              </div>
            </div>
          </div>

          {/* Master-Detail Split: Runs History (Left) + Terminal Log Stream (Right) */}
          <div className="grid grid-cols-12 gap-3 pt-1">
            {/* Left 4 Cols: Recent Runs List */}
            <div className="col-span-4 flex flex-col gap-1.5 border border-[var(--color-ink)] bg-[var(--color-paper)] p-2">
              <span className="font-bold text-xs uppercase tracking-wider text-[var(--color-ink)] border-b border-[var(--color-ink)]/30 pb-1">
                EXECUTION RUNS (LAST 10)
              </span>

              {isLoadingRuns ? (
                <span className="text-[var(--color-ink-mute)] text-xs p-2">LOADING RUNS…</span>
              ) : sourceRuns.length === 0 ? (
                <span className="text-[var(--color-ink-mute)] text-xs p-2">
                  NO ISOLATED RUNS RECORDED YET. CLICK &quot;RUN WORKFLOW NOW&quot; TO TRIGGER.
                </span>
              ) : (
                <div className="flex flex-col divide-y divide-[var(--color-ink)]/20 max-h-64 overflow-y-auto">
                  {sourceRuns.map((r) => {
                    const isSelected = selectedRunId === r.id;
                    return (
                      <div
                        key={r.id}
                        onClick={() => setSelectedRunId(r.id)}
                        className={`p-2 flex flex-col gap-0.5 cursor-pointer text-xs transition-none ${
                          isSelected
                            ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                            : "hover:bg-[var(--color-paper-2)]"
                        }`}
                      >
                        <div className="flex flex-wrap gap-3 items-center justify-between">
                          <div className="flex items-center gap-1.5">
                            <span
                              className={`px-1 py-0.2 text-[9px] uppercase font-bold border ${
                                r.status === "success"
                                  ? "bg-[var(--color-lime)] text-[var(--color-accent-ink)] border-[var(--color-ink)]"
                                  : r.status === "running"
                                  ? "bg-[var(--color-paper-2)] text-[var(--color-ink)] border-[var(--color-ink)] animate-pulse"
                                  : "bg-[var(--color-danger)] text-[var(--color-paper)] border-[var(--color-danger)]"
                              }`}
                            >
                              {r.status}
                            </span>
                            <span className="text-xs">
                              {formatDateTime(r.started_at, settings)}
                            </span>
                          </div>
                          <span className="text-xs">
                            {r.status === "running" ? "RUNNING…" : `+${Number(r.jobs_new)}`}
                          </span>
                        </div>
                        {r.counts && <p className="text-xs">Fetched {r.counts.fetched} · rejected {r.counts.rejected} · blocked {r.counts.blocked} · inserted {r.counts.inserted} · updated {r.counts.updated} · failed {r.counts.failed} · retention removed {r.counts.retention_removed}</p>}
                        {r.error && (
                          <span className={`text-xs truncate ${isSelected ? "text-[var(--color-danger)]" : "text-[var(--color-danger)]"}`}>
                            ERR: {r.error}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Right 8 Cols: Terminal Execution Log Viewer */}
            <div className="col-span-8 flex flex-col gap-1.5 border border-[var(--color-ink)] bg-[var(--color-paper)] p-2">
              <div className="flex items-center justify-between border-b border-[var(--color-ink)]/30 pb-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-bold text-xs uppercase tracking-wider text-[var(--color-ink)]">
                    TERMINAL LOG STREAM {selectedRun ? `· RUN ${selectedRun.id.slice(0, 8)}` : ""}
                  </span>
                  {selectedRun?.status === "running" && (
                    <span className="px-1.5 py-0.2 text-[9px] bg-[var(--color-paper-2)] text-[var(--color-ink)] border border-[var(--color-ink)] animate-pulse font-mono font-bold">
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
                <div className="flex flex-col gap-1 max-h-64 overflow-y-auto font-mono text-xs p-1 bg-[var(--color-paper-2)]/20 border border-[var(--color-ink)]/20">
                  {displayedLogs.map((log) => {
                    const isError = log.level === "error";
                    const isWarn = log.level === "warn";
                    return (
                      <div
                        key={log.id}
                        className={`p-1.5 border text-xs leading-relaxed ${
                          isError
                            ? "border-[var(--color-danger)] bg-[var(--color-paper-2)] text-[var(--color-danger)]"
                            : isWarn
                            ? "border-[var(--color-warning)] bg-[var(--color-paper-2)] text-[var(--color-warning)]"
                            : "border-[var(--color-ink)]/20 bg-[var(--color-paper)] text-[var(--color-ink)]"
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
      )}

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
