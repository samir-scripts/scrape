import { useState, useEffect, useMemo } from "react";
import type { Source } from "../../../bindings/Source";
import type { ScrapeRun } from "../../../bindings/ScrapeRun";
import type { ScrapeLogEntry } from "../../../bindings/ScrapeLogEntry";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import type { Settings } from "../../../bindings/Settings";
import { api } from "../../../lib/api";
import { useUIStore } from "../../../lib/store";
import { formatDateTime, formatTime } from "../../../lib/formatters";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { Tooltip } from "../../ui/Tooltip";
import { AddSourceModal } from "./AddSourceModal";
import { notifySuccess, notifyInfo } from "../../../lib/toast";

type FilterTab = "all" | "ats" | "custom" | "errors";

interface SourcesSettingsProps {
  settings?: Settings;
}

export function SourcesSettings({ settings }: SourcesSettingsProps = {}) {
  const [sources, setSources] = useState<Source[]>([]);
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<FilterTab>("all");

  // Modals & drawers
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
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
  const [logFilter, setLogFilter] = useState<"all" | "errors">("all");

  const { setView, setFilterSpec } = useUIStore();

  const fetchSources = () => {
    api.listSources().then(setSources).catch(() => {});
  };

  useEffect(() => {
    fetchSources();
  }, []);

  // Fetch runs when inspecting a source
  useEffect(() => {
    if (!inspectedSource) {
      setSourceRuns([]);
      setSelectedRunId(null);
      setRunLogs([]);
      return;
    }
    setIsLoadingRuns(true);
    api.listSourceRuns(inspectedSource.id, 10)
      .then((runs) => {
        setSourceRuns(runs);
        if (runs.length > 0) {
          setSelectedRunId((prev) => (prev && runs.some((r) => r.id === prev) ? prev : runs[0].id));
        } else {
          setSelectedRunId(null);
        }
      })
      .catch(() => {
        setSourceRuns([]);
        setSelectedRunId(null);
      })
      .finally(() => setIsLoadingRuns(false));
  }, [inspectedSource]);

  // Fetch logs for the selected run
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
    await api.updateSource({
      id: source.id,
      enabled,
    });
    fetchSources();
  };

  const handleDeleteSource = async (id: string) => {
    if (window.confirm("ARE YOU SURE YOU WANT TO DELETE THIS SCRAPE WORKFLOW?")) {
      await api.deleteSource(id);
      if (inspectedSource?.id === id) setInspectedSource(null);
      fetchSources();
    }
  };

  const handleRunSourceNow = async (source: Source) => {
    try {
      setRunningSourceId(source.id);
      await api.runSourceNow(source.id);
      fetchSources();
      if (inspectedSource?.id === source.id) {
        api.listSourceRuns(source.id, 10).then((runs) => {
          setSourceRuns(runs);
          if (runs[0]) setSelectedRunId(runs[0].id);
        }).catch(() => {});
      }
    } catch (e) {
      console.error("Run single source error:", e);
    } finally {
      setRunningSourceId(null);
    }
  };

  const handleTestSource = async (id: string) => {
    try {
      setTestingSourceId(id);
      setTestEvents(null);
      const events = await api.testSource(id);
      setTestEvents(events);
    } catch (e) {
      console.error("Test source error:", e);
    } finally {
      setTestingSourceId(null);
    }
  };

  const handlePruneExpired = async () => {
    try {
      const pruned = await api.cleanupExpiredJobs();
      notifySuccess(`Pruned ${pruned} expired postings according to TTL policies.`, "TTL PRUNE COMPLETE");
      fetchSources();
    } catch (_e) {
      notifyInfo("No expired postings eligible for pruning.", "TTL RETENTION");
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

  const handleViewInInbox = (companyName: string) => {
    setFilterSpec((prev) => ({ ...prev, search: companyName }));
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

  // Filter logs based on level toggle
  const displayedLogs = useMemo(() => {
    if (logFilter === "errors") {
      return runLogs.filter((l) => l.level === "error" || l.level === "warn");
    }
    return runLogs;
  }, [runLogs, logFilter]);

  const selectedRun = useMemo(() => {
    return sourceRuns.find((r) => r.id === selectedRunId) || null;
  }, [sourceRuns, selectedRunId]);

  return (
    <div className="flex flex-col gap-5 w-full select-none font-mono">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
            WORKFLOWS REGISTRY ({sources.length})
          </h2>
          <span className="text-[11px] text-[var(--color-ink-mute)] uppercase">
            AUTONOMOUS STATIC WORKFLOWS & CONSOLIDATED EXECUTION LOGS
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Tooltip content="Seed or restore the sample Software Engineering Internships workflow">
            <Button
              variant="default"
              onClick={handleSeedSampleWorkflow}
            >
              + SAMPLE SWE
            </Button>
          </Tooltip>

          <Tooltip content="Prune untracked postings past retention TTL. Favorited and tracked jobs are strictly protected.">
            <Button
              variant="default"
              onClick={handlePruneExpired}
            >
              CLEANUP EXPIRED
            </Button>
          </Tooltip>

          <Tooltip content="Create a new autonomous scrape workflow for a company careers site">
            <Button
              variant="inverted"
              onClick={() => {
                setEditingSource(null);
                setIsAddModalOpen(true);
              }}
              shortcut="+ ADD"
            >
              ADD WORKFLOW
            </Button>
          </Tooltip>
        </div>
      </div>

      {/* Sub-Filter Segmented Row + Search */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex gap-1">
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
                ? "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)]"
                : "bg-[var(--color-paper)] text-[var(--color-cherry)] border-[var(--color-cherry)] hover:bg-[var(--color-cherry)]/10"
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
              The application ships with a pre-configured Software Engineering Internships workflow for Cloudflare so you can immediately see autonomous scraping in action.
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
              const isRunning = runningSourceId === s.id;
              const isTesting = testingSourceId === s.id;
              const isInspected = inspectedSource?.id === s.id;

              return (
                <TableRow
                  key={s.id}
                  className={isInspected ? "bg-[var(--color-paper-2)]/60 font-semibold" : ""}
                >
                  <TableCell>
                    <Toggle
                      checked={s.enabled}
                      onChange={(checked) => handleToggleSource(s, checked)}
                    />
                  </TableCell>
                  <TableCell>
                    <div
                      onClick={() => setInspectedSource(isInspected ? null : s)}
                      className="flex flex-col cursor-pointer group"
                    >
                      <span className="font-bold uppercase tracking-tight text-[var(--color-ink)] group-hover:underline">
                        {s.name}
                      </span>
                      <span className="text-[10px] text-[var(--color-ink-mute)] truncate max-w-[240px]">
                        {s.identifier}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="px-1.5 py-0.5 text-[10px] font-mono uppercase border border-[var(--color-ink)] bg-[var(--color-paper-2)]">
                      {s.kind}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="px-1.5 py-0.5 text-[10px] font-mono uppercase font-bold border border-[var(--color-ink)]">
                      {s.cadence === "cron" ? "CRON" : s.cadence.replace("_", " ")}
                    </span>
                  </TableCell>
                  <TableCell className="text-[10px] font-mono whitespace-nowrap">
                    {Number(s.retention_days) === 0
                      ? "FOREVER"
                      : `${s.retention_days}D (${s.retention_policy === "untracked_only" ? "SAFE" : "ALL"})`}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5">
                      <span
                        className={`text-xs ${
                          isRunning
                            ? "text-[var(--color-signal)] animate-pulse"
                            : s.last_status === "ok" || s.last_status === "success"
                            ? "text-[var(--color-lime)]"
                            : s.last_status === "error"
                            ? "text-[var(--color-cherry)]"
                            : "text-[var(--color-ink-mute)]"
                        }`}
                      >
                        ■
                      </span>
                      <span className="text-[10px] font-mono uppercase font-bold text-[var(--color-ink)]">
                        {isRunning ? "RUNNING" : s.last_status || "UNTESTED"}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1.5">
                      <Tooltip content="Run this workflow immediately into database">
                        <button
                          onClick={() => handleRunSourceNow(s)}
                          disabled={isRunning}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-80 text-[10px] font-mono uppercase cursor-pointer disabled:opacity-50"
                        >
                          {isRunning ? "RUNNING…" : "RUN"}
                        </button>
                      </Tooltip>

                      <Tooltip content="Dry run extraction without writing to database">
                        <button
                          onClick={() => handleTestSource(s.id)}
                          disabled={isTesting}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] font-mono uppercase cursor-pointer"
                        >
                          {isTesting ? "…" : "TEST"}
                        </button>
                      </Tooltip>

                      <Tooltip content="View run history, duration telemetry, and live execution logs">
                        <button
                          onClick={() => setInspectedSource(isInspected ? null : s)}
                          className={`px-2 py-0.5 border text-[10px] font-mono uppercase cursor-pointer ${
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
                            setEditingSource(s);
                            setIsAddModalOpen(true);
                          }}
                          className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] font-mono uppercase cursor-pointer"
                        >
                          EDIT
                        </button>
                      </Tooltip>

                      <Tooltip content="Delete scrape workflow">
                        <button
                          onClick={() => handleDeleteSource(s.id)}
                          className="px-2 py-0.5 border border-[var(--color-cherry)] text-[var(--color-cherry)] hover:bg-[var(--color-cherry)] hover:text-white text-[10px] font-mono uppercase cursor-pointer"
                        >
                          DEL
                        </button>
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
            <div className="flex items-center gap-2">
              <span className="text-sm uppercase tracking-tight text-[var(--color-ink)]">
                {inspectedSource.name} // WORKFLOW OBSERVABILITY & EXECUTION LOGS
              </span>
              <span className="px-1.5 py-0.5 text-[10px] border border-[var(--color-ink)] bg-[var(--color-paper)] uppercase">
                {inspectedSource.kind}
              </span>
              <span className="px-1.5 py-0.5 text-[10px] border border-[var(--color-ink)] bg-[var(--color-paper)] uppercase">
                {inspectedSource.cadence}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => handleRunSourceNow(inspectedSource)}
                disabled={runningSourceId === inspectedSource.id}
                className="px-2.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-85 text-[10px] uppercase font-bold cursor-pointer disabled:opacity-50"
              >
                {runningSourceId === inspectedSource.id ? "RUNNING WORKFLOW…" : "RUN WORKFLOW NOW"}
              </button>
              <button
                onClick={() => handleViewInInbox(inspectedSource.name)}
                className="px-2.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] uppercase font-bold cursor-pointer"
              >
                VIEW JOBS IN INBOX →
              </button>
              <button
                onClick={() => setInspectedSource(null)}
                className="text-xs hover:opacity-75 cursor-pointer ml-1"
              >
                [CLOSE]
              </button>
            </div>
          </div>

          {/* Telemetry Row */}
          <div className="grid grid-cols-4 gap-2 p-2 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[11px]">
            <div>
              <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">TARGET URL:</span>
              <div className="font-mono truncate">{inspectedSource.identifier}</div>
            </div>
            <div>
              <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">CADENCE / NEXT RUN:</span>
              <div className="font-mono">
                {inspectedSource.cadence.toUpperCase()}
                {inspectedSource.next_run_at ? ` · ${inspectedSource.next_run_at.slice(0, 16)}` : ""}
              </div>
            </div>
            <div>
              <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">LAST DURATION:</span>
              <div className="font-mono">
                {inspectedSource.last_run_duration_ms
                  ? `${(Number(inspectedSource.last_run_duration_ms) / 1000).toFixed(2)}s`
                  : "NONE"}
              </div>
            </div>
            <div>
              <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">TTL RETENTION:</span>
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
              <span className="font-bold text-[10px] uppercase tracking-wider text-[var(--color-ink)] border-b border-[var(--color-ink)]/30 pb-1">
                EXECUTION RUNS (LAST 10)
              </span>

              {isLoadingRuns ? (
                <span className="text-[var(--color-ink-mute)] text-[11px] p-2">LOADING RUNS…</span>
              ) : sourceRuns.length === 0 ? (
                <span className="text-[var(--color-ink-mute)] text-[11px] p-2">
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
                        className={`p-2 flex flex-col gap-0.5 cursor-pointer text-[11px] transition-none ${
                          isSelected
                            ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                            : "hover:bg-[var(--color-paper-2)]"
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-1.5">
                            <span
                              className={`px-1 py-0.2 text-[9px] uppercase font-bold border ${
                                r.status === "success"
                                  ? "bg-[var(--color-lime)] text-[var(--color-ink)] border-[var(--color-ink)]"
                                  : "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)]"
                              }`}
                            >
                              {r.status}
                            </span>
                            <span className="text-[10px]">
                              {formatDateTime(r.started_at, settings)}
                            </span>
                          </div>
                          <span className="text-[10px]">
                            +{Number(r.jobs_new)}
                          </span>
                        </div>
                        {r.error && (
                          <span className={`text-[10px] truncate ${isSelected ? "text-red-300" : "text-[var(--color-cherry)]"}`}>
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
                <span className="font-bold text-[10px] uppercase tracking-wider text-[var(--color-ink)]">
                  TERMINAL LOG STREAM {selectedRun ? `· RUN ${selectedRun.id.slice(0, 8)}` : ""}
                </span>
                <div className="flex gap-1 text-[10px]">
                  <button
                    onClick={() => setLogFilter("all")}
                    className={`px-2 py-0.5 border cursor-pointer ${
                      logFilter === "all"
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)] font-bold"
                        : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                    }`}
                  >
                    ALL LOGS
                  </button>
                  <button
                    onClick={() => setLogFilter("errors")}
                    className={`px-2 py-0.5 border cursor-pointer ${
                      logFilter === "errors"
                        ? "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)] font-bold"
                        : "border-[var(--color-cherry)] text-[var(--color-cherry)] hover:bg-[var(--color-cherry)]/10"
                    }`}
                  >
                    ERRORS ONLY
                  </button>
                </div>
              </div>

              {isLoadingLogs ? (
                <div className="p-4 text-center text-[11px] text-[var(--color-ink-mute)] font-mono">
                  STREAMING LOGS…
                </div>
              ) : !selectedRunId ? (
                <div className="p-4 text-center text-[11px] text-[var(--color-ink-mute)] font-mono">
                  SELECT AN EXECUTION RUN ON THE LEFT TO INSPECT ITS TERMINAL LOGS.
                </div>
              ) : displayedLogs.length === 0 ? (
                <div className="p-4 text-center text-[11px] text-[var(--color-ink-mute)] font-mono">
                  NO LOG EVENTS RECORDED FOR THIS RUN.
                </div>
              ) : (
                <div className="flex flex-col gap-1 max-h-64 overflow-y-auto font-mono text-[11px] p-1 bg-[var(--color-paper-2)]/20 border border-[var(--color-ink)]/20">
                  {displayedLogs.map((log) => {
                    const isError = log.level === "error";
                    const isWarn = log.level === "warn";
                    return (
                      <div
                        key={log.id}
                        className={`p-1.5 border text-[11px] leading-relaxed ${
                          isError
                            ? "border-red-600 bg-red-50/50 text-red-900"
                            : isWarn
                            ? "border-amber-600 bg-amber-50/50 text-amber-900"
                            : "border-[var(--color-ink)]/20 bg-[var(--color-paper)] text-[var(--color-ink)]"
                        }`}
                      >
                        <div className="flex items-center justify-between text-[10px] opacity-80 mb-0.5 font-bold">
                          <span className={isError ? "text-red-700" : isWarn ? "text-amber-700" : "text-[var(--color-ink)]"}>
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
                <div key={i} className="text-[11px] truncate">
                  {ev.type === "job" ? (
                    <span>
                      <strong className="text-[var(--color-ink)]">{ev.title}</strong> · {ev.company} · {ev.location || "REMOTE"} ({ev.role_type})
                      {ev.salary?.raw ? ` · ${ev.salary.raw}` : ""}
                    </span>
                  ) : ev.type === "error" ? (
                    <span className="text-[var(--color-cherry)]">
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
        }}
        onSourceSaved={fetchSources}
        initialSource={editingSource}
        settings={settings}
      />
    </div>
  );
}
