import { useState, useEffect } from "react";
import type { Settings } from "../../../bindings/Settings";
import type { ScrapeRun } from "../../../bindings/ScrapeRun";
import type { CronValidationResult } from "../../../bindings/CronValidationResult";
import { api } from "../../../lib/api";
import { Toggle } from "../../ui/Toggle";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { Table, TableHead, TableHeader, TableBody, TableRow, TableCell } from "../../ui/Table";
import { Rule } from "../../ui/Rule";
import { CRON_PRESETS } from "../../../lib/constants";

interface ScheduleSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function ScheduleSettings({
  settings,
  onUpdateSetting,
}: ScheduleSettingsProps) {
  const [cronInput, setCronInput] = useState(settings.scraper_cron);
  const [validation, setValidation] = useState<CronValidationResult | null>(null);
  const [recentRuns, setRecentRuns] = useState<ScrapeRun[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  // Validate cron whenever input changes
  useEffect(() => {
    let cancelled = false;
    api.validateCron(cronInput).then((res) => {
      if (!cancelled) setValidation(res);
    });
    return () => {
      cancelled = true;
    };
  }, [cronInput]);

  // Load recent runs
  const fetchRecentRuns = () => {
    api.listScrapeRuns(8).then(setRecentRuns).catch(() => {});
  };

  useEffect(() => {
    fetchRecentRuns();
  }, []);

  const handleApplyCron = (newCron: string) => {
    setCronInput(newCron);
    onUpdateSetting("scraper.cron", newCron);
  };

  const handleRunNow = async () => {
    try {
      setIsRunning(true);
      await api.runScrapeNow();
      fetchRecentRuns();
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
            SCHEDULE
          </h2>
        </div>
        <Button
          variant="inverted"
          onClick={handleRunNow}
          disabled={isRunning}
          shortcut="RUN"
        >
          {isRunning ? "RUNNING…" : "RUN SCRAPE NOW"}
        </Button>
      </div>

      <Rule />

      {/* Main Toggles */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          checked={settings.scraper_enabled}
          onChange={(checked) => onUpdateSetting("scraper.enabled", checked)}
          label="SCHEDULED SCRAPING"
        />
        <Toggle
          checked={settings.scraper_run_on_startup}
          onChange={(checked) =>
            onUpdateSetting("scraper.run_on_startup", checked)
          }
          label="RUN ON STARTUP"
        />
      </div>

      {/* Cron Presets & Expression */}
      <div className="flex flex-col gap-3">
        <label className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
          PRESETS
        </label>
        <div className="grid grid-cols-4 gap-2">
          {CRON_PRESETS.map((preset) => {
            const isSelected = settings.scraper_cron === preset.expr;
            return (
              <button
                key={preset.label}
                onClick={() => handleApplyCron(preset.expr)}
                className={`px-3 py-2 border font-mono text-xs font-bold uppercase tracking-wider cursor-pointer text-center ${
                  isSelected
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-transparent text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {preset.label}
              </button>
            );
          })}
        </div>

        <div className="flex flex-col gap-1.5 mt-2">
          <label className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
            CUSTOM CRON
          </label>
          <div className="flex gap-2">
            <Input
              variant="mono"
              value={cronInput}
              onChange={(e) => setCronInput(e.target.value)}
              placeholder="0 0 */6 * * *"
              className="flex-1"
            />
            <Button
              variant="default"
              onClick={() => onUpdateSetting("scraper.cron", cronInput)}
              disabled={!validation?.ok || cronInput === settings.scraper_cron}
            >
              SAVE CRON
            </Button>
          </div>

          {/* Validation & Live Preview */}
          {validation && (
            <div className="mt-1 font-mono text-xs">
              {validation.ok ? (
                <div className="flex flex-col gap-1 text-[11px] text-[var(--color-ink-mute)]">
                  <span className="text-[var(--color-lime)] font-bold">
                    ✓ VALID
                  </span>
                  <span>NEXT RUNS:</span>
                  <ul className="list-disc list-inside pl-1 text-[var(--color-ink)]">
                    {validation.next_runs.map((r, i) => (
                      <li key={i}>{r}</li>
                    ))}
                  </ul>
                </div>
              ) : (
                <span className="text-[var(--color-cherry)] text-[11px] font-bold">
                  ✗ {validation.error || "INVALID CRON EXPRESSION"}
                </span>
              )}
            </div>
          )}
        </div>
      </div>

      <Rule />

      {/* Recent Scrape Runs Table */}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between items-center">
          <span className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
            RECENT RUNS
          </span>
          <button
            onClick={fetchRecentRuns}
            className="text-[10px] font-mono uppercase text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] cursor-pointer"
          >
            REFRESH ↻
          </button>
        </div>

        {recentRuns.length === 0 ? (
          <div className="p-4 border border-dashed border-[var(--color-ink-mute)] text-center text-xs font-mono text-[var(--color-ink-mute)]">
            NO RECENT RUNS FOUND
          </div>
        ) : (
          <Table>
            <TableHead>
              <TableRow>
                <TableHeader>TRIGGER</TableHeader>
                <TableHeader>STATUS</TableHeader>
                <TableHeader>STARTED</TableHeader>
                <TableHeader>JOBS FOUND</TableHeader>
                <TableHeader>NEW JOBS</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {recentRuns.map((r) => (
                <TableRow key={r.id}>
                  <TableCell className="font-bold uppercase">
                    {r.trigger}
                  </TableCell>
                  <TableCell>
                    <span
                      className={`px-1.5 py-0.5 text-[10px] font-mono uppercase font-bold border ${
                        r.status === "success"
                          ? "bg-[var(--color-lime)] text-[var(--color-ink)] border-[var(--color-ink)]"
                          : r.status === "failed"
                          ? "bg-[var(--color-cherry)] text-white border-[var(--color-cherry)]"
                          : "bg-[var(--color-lavender)] text-[var(--color-ink)] border-[var(--color-ink)]"
                      }`}
                    >
                      {r.status}
                    </span>
                  </TableCell>
                  <TableCell className="text-[var(--color-ink-mute)]">
                    {r.started_at ? r.started_at.slice(0, 19).replace("T", " ") : "-"}
                  </TableCell>
                  <TableCell className="font-mono">{r.jobs_found}</TableCell>
                  <TableCell className="font-mono font-bold">
                    {r.jobs_new}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </div>
    </div>
  );
}
