import { useState, useEffect } from "react";
import type { Source } from "../../../bindings/Source";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import { api } from "../../../lib/api";
import { Button } from "../../ui/Button";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { AddSourceModal } from "./AddSourceModal";

export function SourcesSettings() {
  const [sources, setSources] = useState<Source[]>([]);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [testingSourceId, setTestingSourceId] = useState<string | null>(null);
  const [testEvents, setTestEvents] = useState<WorkerEvent[] | null>(null);

  const fetchSources = () => {
    api.listSources().then(setSources).catch(() => {});
  };

  useEffect(() => {
    fetchSources();
  }, []);

  const handleToggleSource = async (source: Source, enabled: boolean) => {
    await api.updateSource({
      id: source.id,
      enabled,
    });
    fetchSources();
  };

  const handleDeleteSource = async (id: string) => {
    if (window.confirm("ARE YOU SURE YOU WANT TO DELETE THIS SOURCE?")) {
      await api.deleteSource(id);
      fetchSources();
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

  return (
    <div className="flex flex-col gap-6 w-full max-w-4xl select-none">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
            SOURCES
          </h2>
        </div>
        <Button
          variant="inverted"
          onClick={() => setIsAddModalOpen(true)}
          shortcut="+ ADD"
        >
          ADD SOURCE
        </Button>
      </div>

      <Rule />

      {/* Sources Table */}
      {sources.length === 0 ? (
        <div className="p-8 border border-dashed border-[var(--color-ink-mute)] text-center text-xs font-mono text-[var(--color-ink-mute)]">
          NO CAREER SOURCES CONFIGURED. CLICK &quot;ADD SOURCE&quot; TO CONFIGURE A TARGET.
        </div>
      ) : (
        <Table>
          <TableHead>
            <TableRow>
              <TableHeader>ENABLED</TableHeader>
              <TableHeader>NAME</TableHeader>
              <TableHeader>KIND</TableHeader>
              <TableHeader>IDENTIFIER</TableHeader>
              <TableHeader>LAST RESULT</TableHeader>
              <TableHeader>ACTIONS</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {sources.map((s) => (
              <TableRow key={s.id}>
                <TableCell>
                  <Toggle
                    checked={s.enabled}
                    onChange={(checked) => handleToggleSource(s, checked)}
                  />
                </TableCell>
                <TableCell className="font-bold uppercase">{s.name}</TableCell>
                <TableCell className="font-mono text-[10px] uppercase">
                  {s.kind}
                </TableCell>
                <TableCell className="font-mono text-[11px] truncate max-w-[200px]">
                  {s.identifier}
                </TableCell>
                <TableCell>
                  <span
                    className={`px-1.5 py-0.5 text-[10px] font-mono uppercase font-bold border ${
                      s.last_status === "success"
                        ? "bg-[var(--color-lime)] text-[var(--color-ink)] border-[var(--color-ink)]"
                        : s.last_status === "error"
                        ? "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)]"
                        : "bg-[var(--color-paper-2)] text-[var(--color-ink-mute)] border-[var(--color-ink-mute)]"
                    }`}
                  >
                    {s.last_status || "UNTESTED"}
                  </span>
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleTestSource(s.id)}
                      disabled={testingSourceId === s.id}
                      className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] text-[10px] font-mono uppercase cursor-pointer"
                    >
                      {testingSourceId === s.id ? "TESTING…" : "TEST"}
                    </button>
                    <button
                      onClick={() => handleDeleteSource(s.id)}
                      className="px-2 py-0.5 border border-[var(--color-cherry)] text-[var(--color-cherry)] hover:bg-[var(--color-cherry)] hover:text-white text-[10px] font-mono uppercase cursor-pointer"
                    >
                      DELETE
                    </button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      {/* Test Results Preview Drawer */}
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
        onClose={() => setIsAddModalOpen(false)}
        onSourceCreated={fetchSources}
      />
    </div>
  );
}
