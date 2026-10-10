import { useState, useEffect, useMemo } from "react";
import type { Source } from "../../../bindings/Source";
import type { ScrapeRun } from "../../../bindings/ScrapeRun";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import { api } from "../../../lib/api";
import { useUIStore } from "../../../lib/store";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { AddSourceModal } from "./AddSourceModal";
import { notifySuccess, notifyInfo } from "../../../lib/toast";

type FilterTab = "all" | "ats" | "custom" | "errors";

export function SourcesSettings() {
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

  // Inspector Drawer state
  const [inspectedSource, setInspectedSource] = useState<Source | null>(null);
  const [sourceRuns, setSourceRuns] = useState<ScrapeRun[]>([]);
  const [isLoadingRuns, setIsLoadingRuns] = useState(false);

  const { setView, setSearchQuery } = useUIStore();

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
      return;
    }
    setIsLoadingRuns(true);
    api.listSourceRuns(inspectedSource.id, 5)
      .then(setSourceRuns)
      .catch(() => setSourceRuns([]))
      .finally(() => setIsLoadingRuns(false));
  }, [inspectedSource]);

  const handleToggleSource = async (source: Source, enabled: boolean) => {
    await api.updateSource({
      id: source.id,
      enabled,
    });
    fetchSources();
  };

  const handleDeleteSource = async (id: string) => {
    if (window.confirm("ARE YOU SURE YOU WANT TO DELETE THIS SCRAPE JOB?")) {
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
        api.listSourceRuns(source.id, 5).then(setSourceRuns).catch(() => {});
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
    } catch (e) {
      notifyInfo("No expired postings eligible for pruning.", "TTL RETENTION");
    }
  };

  const handleViewInInbox = (companyName: string) => {
    setSearchQuery(companyName);
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

  return (
    <div className="flex flex-col gap-6 w-full max-w-5xl select-none font-mono">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
            SCRAPE JOBS REGISTRY ({sources.length})
          </h2>
          <span className="text-[11px] text-[var(--color-ink-mute)] uppercase">
            TARGET COMPANY CAREER PORTALS & INDEPENDENT EXECUTION CADENCES
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="default"
            onClick={handlePruneExpired}
            title="Prune expired untracked postings according to TTL retention policies"
          >
            CLEANUP EXPIRED
          </Button>
          <Button
            variant="inverted"
            onClick={() => {
              setEditingSource(null);
              setIsAddModalOpen(true);
            }}
            shortcut="+ ADD JOB"
          >
            ADD WORKFLOW
          </Button>
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
            placeholder="SEARCH JOBS..."
            className="text-xs"
          />
        </div>
      </div>

      <Rule />

      {/* Scrape Jobs Registry Table */}
      {filteredSources.length === 0 ? (
        <div className="p-8 border border-dashed border-[var(--color-ink-mute)] text-center text-xs font-mono text-[var(--color-ink-mute)]">
          NO MATCHING SCRAPE WORKFLOWS FOUND. CLICK &quot;ADD SCRAPE JOB&quot; TO CONFIGURE A TARGET.
        </div>
      ) : (
        <Table>
          <TableHead>
            <TableRow>
              <TableHeader>ON</TableHeader>
              <TableHeader>WORKFLOW DAG</TableHeader>
              <TableHeader>KIND</TableHeader>
              <TableHeader>CADENCE</TableHeader>
              <TableHeader>RETENTION</TableHeader>
              <TableHeader>STATUS</TableHeader>
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
                    <div className="flex flex-col">
                      <span className="font-bold uppercase tracking-tight text-[var(--color-ink)]">
                        {s.name}
                      </span>
                      <span className="text-[10px] text-[var(--color-ink-mute)] truncate max-w-[200px]">
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
                      <button
                        onClick={() => handleRunSourceNow(s)}
                        disabled={isRunning}
                        title="Run this scrape job immediately into database"
                        className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-80 text-[10px] font-mono uppercase cursor-pointer disabled:opacity-50"
                      >
                        {isRunning ? "RUNNING…" : "RUN"}
                      </button>
                      <button
                        onClick={() => handleTestSource(s.id)}
                        disabled={isTesting}
                        title="Dry run test without saving jobs"
                        className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] font-mono uppercase cursor-pointer"
                      >
                        {isTesting ? "…" : "TEST"}
                      </button>
                      <button
                        onClick={() => {
                          setEditingSource(s);
                          setIsAddModalOpen(true);
                        }}
                        title="Edit selectors, cadence, or target URL"
                        className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] font-mono uppercase cursor-pointer"
                      >
                        EDIT
                      </button>
                      <button
                        onClick={() => setInspectedSource(isInspected ? null : s)}
                        title="Inspect run history and telemetry"
                        className={`px-2 py-0.5 border text-[10px] font-mono uppercase cursor-pointer ${
                          isInspected
                            ? "border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                            : "border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)]"
                        }`}
                      >
                        {isInspected ? "HIDE" : "INSPECT"}
                      </button>
                      <button
                        onClick={() => handleDeleteSource(s.id)}
                        title="Delete scrape job"
                        className="px-2 py-0.5 border border-[var(--color-cherry)] text-[var(--color-cherry)] hover:bg-[var(--color-cherry)] hover:text-white text-[10px] font-mono uppercase cursor-pointer"
                      >
                        DEL
                      </button>
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}

      {/* Telemetry Inspector Drawer */}
      {inspectedSource && (
        <div className="flex flex-col gap-3 p-4 border-2 border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 text-xs">
          <div className="flex justify-between items-center border-b border-[var(--color-ink)] pb-2 font-bold">
            <div className="flex items-center gap-2">
              <span className="text-sm uppercase tracking-tight text-[var(--color-ink)]">
                {inspectedSource.name} // TELEMETRY & RUN HISTORY
              </span>
              <span className="px-1.5 py-0.5 text-[10px] border border-[var(--color-ink)] bg-[var(--color-paper)] uppercase">
                {inspectedSource.kind}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => handleViewInInbox(inspectedSource.name)}
                className="px-2.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-85 text-[10px] uppercase font-bold cursor-pointer"
              >
                VIEW JOBS IN INBOX →
              </button>
              <button
                onClick={() => setInspectedSource(null)}
                className="text-xs hover:opacity-75 cursor-pointer"
              >
                [CLOSE]
              </button>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[11px]">
            <div>
              <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">TARGET URL / SLUG:</span>
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
          </div>

          {/* Recent 5 runs */}
          <div className="flex flex-col gap-1.5">
            <span className="font-bold text-[10px] uppercase tracking-wider text-[var(--color-ink)]">
              RECENT EXECUTION RUNS (LAST 5)
            </span>
            {isLoadingRuns ? (
              <span className="text-[var(--color-ink-mute)] text-[11px]">LOADING RUNS…</span>
            ) : sourceRuns.length === 0 ? (
              <span className="text-[var(--color-ink-mute)] text-[11px]">
                NO ISOLATED RUNS RECORDED YET FOR THIS JOB.
              </span>
            ) : (
              <div className="flex flex-col border border-[var(--color-ink)] bg-[var(--color-paper)] divide-y divide-[var(--color-ink)]/20">
                {sourceRuns.map((r) => (
                  <div key={r.id} className="p-2 flex items-center justify-between text-[11px]">
                    <div className="flex items-center gap-2">
                      <span
                        className={`px-1 py-0.2 text-[9px] uppercase font-bold border ${
                          r.status === "success"
                            ? "bg-[var(--color-lime)] text-[var(--color-ink)] border-[var(--color-ink)]"
                            : "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)]"
                        }`}
                      >
                        {r.status}
                      </span>
                      <span>{r.started_at.slice(0, 19).replace("T", " ")}</span>
                    </div>
                    <div className="flex items-center gap-4 text-[10px]">
                      <span>+{Number(r.jobs_new)} NEW</span>
                      <span>{Number(r.jobs_found)} FOUND</span>
                      {r.error && (
                        <span className="text-[var(--color-cherry)] truncate max-w-[200px]">
                          ERR: {r.error}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
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
      />
    </div>
  );
}
