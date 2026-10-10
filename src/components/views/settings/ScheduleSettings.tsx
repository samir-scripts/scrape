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
import { ClockPicker } from "../../ui/ClockPicker";
import {
  cronToClock,
  clockToCron,
  type ClockScheduleState,
  CLOCK_CADENCE_OPTIONS,
} from "../../../lib/cronClock";

interface ScheduleSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function ScheduleSettings({
  settings,
  onUpdateSetting,
}: ScheduleSettingsProps) {
  const [scheduleMode, setScheduleMode] = useState<"clock" | "cron">(() => {
    return cronToClock(settings.scraper_cron) ? "clock" : "cron";
  });

  const [cronInput, setCronInput] = useState(settings.scraper_cron);
  const [clockState, setClockState] = useState<ClockScheduleState>(() => {
    return (
      cronToClock(settings.scraper_cron) ?? {
        hour12: 9,
        minute: 0,
        ampm: "AM",
        cadence: "daily",
      }
    );
  });

  const [validation, setValidation] = useState<CronValidationResult | null>(null);
  const [recentRuns, setRecentRuns] = useState<ScrapeRun[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  // Synchronize when settings.scraper_cron changes externally
  useEffect(() => {
    setCronInput(settings.scraper_cron);
    const parsed = cronToClock(settings.scraper_cron);
    if (parsed) {
      setClockState(parsed);
    }
  }, [settings.scraper_cron]);

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

  const handleClockChange = (state: ClockScheduleState, newCron: string) => {
    setClockState(state);
    setCronInput(newCron);
  };

  const handleApplyClockSchedule = () => {
    const cron = clockToCron(
      clockState.hour12,
      clockState.minute,
      clockState.ampm,
      clockState.cadence
    );
    setCronInput(cron);
    onUpdateSetting("scraper.cron", cron);
  };

  const handleSaveCustomCron = () => {
    onUpdateSetting("scraper.cron", cronInput);
    const parsed = cronToClock(cronInput);
    if (parsed) {
      setClockState(parsed);
    }
  };

  const handleSwitchMode = (mode: "clock" | "cron") => {
    setScheduleMode(mode);
    if (mode === "clock") {
      const parsed = cronToClock(cronInput);
      if (parsed) {
        setClockState(parsed);
      }
    }
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

  const isCustomCronOutOfSyncWithClock =
    scheduleMode === "clock" && !cronToClock(settings.scraper_cron);

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

      {/* Schedule Configuration Modes: Clock Schedule vs Custom Cron */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between border-b border-[var(--color-ink)]">
          <div className="flex">
            <button
              type="button"
              onClick={() => handleSwitchMode("clock")}
              className={`px-4 py-2 font-mono text-xs font-bold uppercase tracking-wider cursor-pointer border-r border-[var(--color-ink)] transition-colors ${
                scheduleMode === "clock"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-transparent text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]/40"
              }`}
            >
              ■ CLOCK SCHEDULE
            </button>
            <button
              type="button"
              onClick={() => handleSwitchMode("cron")}
              className={`px-4 py-2 font-mono text-xs font-bold uppercase tracking-wider cursor-pointer border-r border-[var(--color-ink)] transition-colors ${
                scheduleMode === "cron"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-transparent text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]/40"
              }`}
            >
              ≡ CUSTOM CRON
            </button>
          </div>

          <span className="text-[10px] font-mono text-[var(--color-ink-mute)] pr-1">
            ACTIVE CRON: <code className="font-bold text-[var(--color-ink)]">{settings.scraper_cron}</code>
          </span>
        </div>

        {/* CLOCK SCHEDULE MODE */}
        {scheduleMode === "clock" && (
          <div className="flex flex-col md:flex-row gap-4 items-start">
            <ClockPicker
              value={clockState}
              onChange={handleClockChange}
              className="flex-1"
            />

            {/* Side Action & Validation Panel */}
            <div className="flex flex-col gap-3 w-full md:w-64 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20">
              <div className="flex justify-between items-center text-[10px] font-mono uppercase text-[var(--color-ink-mute)] pb-1 border-b border-[var(--color-ink)]">
                <span>SCHEDULE SUMMARY</span>
                <span className={cronInput === settings.scraper_cron ? "text-[var(--color-lime)] font-bold" : "text-[var(--color-signal)] font-bold"}>
                  {cronInput === settings.scraper_cron ? "CURRENT" : "UNSAVED"}
                </span>
              </div>

              {isCustomCronOutOfSyncWithClock && (
                <div className="p-2 border border-dashed border-[var(--color-cherry)] text-[10px] font-mono text-[var(--color-cherry)]">
                  ACTIVE SETTING USES A CUSTOM CRON FORMAT. APPLYING CLOCK SCHEDULE WILL REPLACE IT.
                </div>
              )}

              <div className="flex flex-col gap-1 text-[11px] font-mono">
                <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">CADENCE</span>
                <span className="font-bold text-[var(--color-ink)]">
                  {CLOCK_CADENCE_OPTIONS.find((c) => c.id === clockState.cadence)?.label ?? clockState.cadence.toUpperCase()}
                </span>
                <span className="text-[10px] text-[var(--color-ink-mute)] mt-1 uppercase">EXPRESSION</span>
                <code className="text-xs font-bold bg-[var(--color-paper)] p-1 border border-[var(--color-ink)] text-[var(--color-ink)]">
                  {cronInput}
                </code>
              </div>

              <Button
                variant={cronInput === settings.scraper_cron ? "default" : "inverted"}
                onClick={handleApplyClockSchedule}
                disabled={!validation?.ok || cronInput === settings.scraper_cron}
                className="w-full text-center justify-center mt-1"
              >
                {cronInput === settings.scraper_cron ? "SCHEDULE SAVED" : "APPLY SCHEDULE"}
              </Button>

              {/* Validation & Next Runs */}
              {validation && (
                <div className="mt-1 pt-2 border-t border-[var(--color-ink)]/40 font-mono text-xs">
                  {validation.ok ? (
                    <div className="flex flex-col gap-1 text-[11px]">
                      <span className="text-[var(--color-lime)] font-bold">
                        ✓ VALID SCHEDULE
                      </span>
                      <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">
                        NEXT RUNS:
                      </span>
                      <ul className="list-disc list-inside pl-0.5 text-[10px] text-[var(--color-ink)] space-y-0.5">
                        {validation.next_runs.map((r, i) => (
                          <li key={i} className="truncate">
                            {r.slice(0, 19).replace("T", " ")}
                          </li>
                        ))}
                      </ul>
                    </div>
                  ) : (
                    <span className="text-[var(--color-cherry)] text-[10px] font-bold">
                      ✗ {validation.error || "INVALID CRON"}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {/* CUSTOM CRON MODE */}
        {scheduleMode === "cron" && (
          <div className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper)]">
            <div className="flex flex-col gap-1.5">
              <div className="flex justify-between items-center">
                <label className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  CUSTOM CRON EXPRESSION
                </label>
                <span className="text-[10px] font-mono text-[var(--color-ink-mute)]">
                  FORMAT: SEC MIN HOUR DOM MON DOW
                </span>
              </div>
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
                  onClick={handleSaveCustomCron}
                  disabled={!validation?.ok || cronInput === settings.scraper_cron}
                >
                  SAVE CRON
                </Button>
              </div>
            </div>

            {/* Field Breakdown Guide */}
            <div className="grid grid-cols-6 gap-1 p-2 bg-[var(--color-paper-2)]/30 border border-[var(--color-ink)] text-[10px] font-mono text-center">
              <div>
                <span className="text-[var(--color-ink-mute)] block">SEC</span>
                <span className="font-bold">0</span>
              </div>
              <div>
                <span className="text-[var(--color-ink-mute)] block">MIN</span>
                <span className="font-bold">0-59</span>
              </div>
              <div>
                <span className="text-[var(--color-ink-mute)] block">HOUR</span>
                <span className="font-bold">0-23</span>
              </div>
              <div>
                <span className="text-[var(--color-ink-mute)] block">DOM</span>
                <span className="font-bold">*</span>
              </div>
              <div>
                <span className="text-[var(--color-ink-mute)] block">MON</span>
                <span className="font-bold">*</span>
              </div>
              <div>
                <span className="text-[var(--color-ink-mute)] block">DOW</span>
                <span className="font-bold">1-5 / *</span>
              </div>
            </div>

            {/* Validation & Live Preview */}
            {validation && (
              <div className="mt-1 font-mono text-xs">
                {validation.ok ? (
                  <div className="flex flex-col gap-1 text-[11px] text-[var(--color-ink-mute)]">
                    <span className="text-[var(--color-lime)] font-bold">
                      ✓ VALID EXPRESSION
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
        )}
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
