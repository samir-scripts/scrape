import { useEffect, useState } from "react";
import type { WorkflowSourceSpec } from "../../../bindings/WorkflowSourceSpec";
import type { ExtractionMode } from "../../../bindings/ExtractionMode";
import type { ScriptRuntime } from "../../../bindings/ScriptRuntime";
import type { ExtractorTestResult } from "../../../bindings/ExtractorTestResult";
import type { BrowserRuntimeStatus } from "../../../bindings/BrowserRuntimeStatus";
import type { BrowserSessionSummary } from "../../../bindings/BrowserSessionSummary";
import type { ScriptEnvironmentStatus } from "../../../bindings/ScriptEnvironmentStatus";
import type { Source } from "../../../bindings/Source";
import type { WorkflowSpec } from "../../../bindings/WorkflowSpec";
import type { WorkflowScriptConfig } from "../../../bindings/WorkflowScriptConfig";
import { Select } from "../../ui/Select";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { api } from "../../../lib/api";

const MODE_OPTIONS = [
  { value: "static", label: "Static HTML · CSS & XPath (Fastest, low memory)" },
  {
    value: "browser",
    label:
      "Headless Browser · CDP (Dynamic SPAs & authenticated sessions)",
  },
  {
    value: "script",
    label: "Custom Script · Stdin/NDJSON (Local JS or Python runner)",
  },
];

const RUNTIME_OPTIONS = [
  { value: "js", label: "JavaScript (Node)" },
  { value: "python", label: "Python 3 (Explicit interpreter)" },
  { value: "custom", label: "Custom executable" },
];

interface CustomExtractorConfigProps {
  source: WorkflowSourceSpec;
  sourceIndex: number;
  onSourceChange: (updated: WorkflowSourceSpec) => void;
  initialSource?: Source | null;
  onTrustChanged?: () => void;
}

export function CustomExtractorConfig({
  source,
  sourceIndex,
  onSourceChange,
  initialSource,
  onTrustChanged,
}: CustomExtractorConfigProps) {
  const extractor = source.extractor;
  const mode: ExtractionMode = extractor.mode || "static";

  // State for Test Extractor
  const [testResult, setTestResult] = useState<ExtractorTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [testError, setTestError] = useState<string | null>(null);

  // State for Browser Sessions & Runtime
  const [browserRuntime, setBrowserRuntime] =
    useState<BrowserRuntimeStatus | null>(null);
  const [sessions, setSessions] = useState<BrowserSessionSummary[]>([]);
  const [sessionActionMsg, setSessionActionMsg] = useState<string | null>(null);
  const [showOpenSession, setShowOpenSession] = useState(false);
  const [openLabel, setOpenLabel] = useState(
    source.label ? `${source.label} Session` : `Source ${sourceIndex + 1} Session`
  );
  const [openUrl, setOpenUrl] = useState(extractor.url || "");
  const [sessionBusy, setSessionBusy] = useState(false);

  // State for Script environment & trust
  const [envStatus, setEnvStatus] = useState<ScriptEnvironmentStatus | null>(
    null
  );
  const [envBusy, setEnvBusy] = useState(false);
  const [envError, setEnvError] = useState<string | null>(null);
  const [trustBusy, setTrustBusy] = useState(false);
  const [trustSuccess, setTrustSuccess] = useState<string | null>(null);
  const [trustError, setTrustError] = useState<string | null>(null);
  const [trustedSource, setTrustedSource] = useState<Source | null>(null);

  const [ownedSource, setOwnedSource] = useState<Source | null>(null);
  useEffect(() => {
    let active = true;
    setTrustedSource(null); setOwnedSource(null); setTrustSuccess(null); setTrustError(null);
    if (initialSource?.id === source.id) setOwnedSource(initialSource);
    else if (initialSource?.id) void api.getSource(source.id).then(value => { if (active) setOwnedSource(value); }).catch(() => {});
    return () => { active = false; };
  }, [initialSource, source.id]);
  const reviewSource = trustedSource ?? ownedSource;

  const savedWorkflowSpec =
    initialSource?.config &&
    typeof initialSource.config === "object" &&
    !Array.isArray(initialSource.config)
      ? ((initialSource.config as Record<string, unknown>).__workflow_spec as WorkflowSpec | undefined)
      : undefined;
  const savedSource = savedWorkflowSpec?.sources?.find((s) => s.id === source.id);
  const ownedConfig = reviewSource?.config && typeof reviewSource.config === "object" && !Array.isArray(reviewSource.config)
    ? reviewSource.config as Record<string, unknown> : undefined;
  const savedScript = (ownedConfig?.script as WorkflowScriptConfig | undefined) ?? savedSource?.extractor?.script;

  const isScriptEqual = (
    a?: WorkflowScriptConfig | null,
    b?: WorkflowScriptConfig | null
  ) => {
    if (!a && !b) return true;
    if (!a || !b) return false;
    const norm = (v: string | null | undefined) => (v ?? "");
    if (a.runtime !== b.runtime) return false;
    if (norm(a.interpreter) !== norm(b.interpreter)) return false;
    if (norm(a.path) !== norm(b.path)) return false;
    if (norm(a.code) !== norm(b.code)) return false;
    if ((a.timeout_s ?? 30) !== (b.timeout_s ?? 30)) return false;
    const argsA = a.args || [];
    const argsB = b.args || [];
    if (argsA.length !== argsB.length) return false;
    for (let i = 0; i < argsA.length; i++) {
      if (argsA[i] !== argsB[i]) return false;
    }
    return true;
  };
  const scriptDirty = !isScriptEqual(savedScript, extractor.script);

  // Load browser runtime and sessions when entering browser mode
  useEffect(() => {
    if (mode === "browser") {
      api.detectBrowserRuntime().then(setBrowserRuntime).catch(() => {});
      api.listBrowserSessions().then(setSessions).catch(() => {});
    }
  }, [mode]);

  const updateExtractor = (
    patch: Partial<WorkflowSourceSpec["extractor"]>
  ) => {
    onSourceChange({
      ...source,
      extractor: {
        ...extractor,
        ...patch,
      },
    });
  };

  const handleModeChange = (newMode: string) => {
    const m = newMode as ExtractionMode;
    const patch: Partial<WorkflowSourceSpec["extractor"]> = { mode: m };
    if (m === "script" && !extractor.script) {
      patch.script = {
        runtime: "js",
        timeout_s: 30,
        args: [],
      };
    }
    updateExtractor(patch);
  };

  const handleTestExtractor = async () => {
    setTesting(true);
    setTestError(null);
    try {
      const res = await api.testExtractor(extractor);
      setTestResult(res);
    } catch (err) {
      setTestError(String(err));
    } finally {
      setTesting(false);
    }
  };

  const handleOpenSession = async () => {
    if (!openUrl.trim()) return;
    setSessionBusy(true);
    setSessionActionMsg(null);
    try {
      const created = await api.openBrowserSession(
        openLabel.trim() || `Session ${Date.now()}`,
        openUrl.trim()
      );
      const list = await api.listBrowserSessions();
      setSessions(list);
      updateExtractor({ session_id: created.id });
      setShowOpenSession(false);
      setSessionActionMsg(
        `Session opened. Log in using the browser window, then click "Save session".`
      );
    } catch (err) {
      setSessionActionMsg(`Failed to open login browser: ${String(err)}`);
    } finally {
      setSessionBusy(false);
    }
  };

  const handleSaveSession = async () => {
    if (!extractor.session_id) return;
    setSessionBusy(true);
    setSessionActionMsg(null);
    try {
      await api.saveBrowserSession(extractor.session_id);
      const list = await api.listBrowserSessions();
      setSessions(list);
      setSessionActionMsg(`Session saved and ready.`);
    } catch (err) {
      setSessionActionMsg(`Failed to save session: ${String(err)}`);
    } finally {
      setSessionBusy(false);
    }
  };

  const handleClearSession = async () => {
    if (!extractor.session_id) return;
    setSessionBusy(true);
    setSessionActionMsg(null);
    try {
      await api.clearBrowserSession(extractor.session_id);
      const list = await api.listBrowserSessions();
      setSessions(list);
      updateExtractor({ session_id: null });
      setSessionActionMsg(`Session cleared.`);
    } catch (err) {
      setSessionActionMsg(`Failed to clear session: ${String(err)}`);
    } finally {
      setSessionBusy(false);
    }
  };

  const handleCheckEnvironment = async () => {
    if (!extractor.script) return;
    setEnvBusy(true);
    setEnvError(null);
    try {
      const res = await api.checkScriptEnvironment(extractor.script);
      setEnvStatus(res);
    } catch (err) {
      setEnvError(String(err));
    } finally {
      setEnvBusy(false);
    }
  };

  const handleTrustScript = async () => {
    if (!reviewSource?.id || !reviewSource.script_sha256) return;
    setTrustBusy(true);
    setTrustError(null);
    try {
      const updated = await api.trustWorkflowScript(
        reviewSource.id,
        reviewSource.script_sha256,
        true
      );
      setTrustedSource(updated);
      setTrustSuccess("Saved script trusted. Enable the workflow explicitly when ready.");
      if (onTrustChanged) onTrustChanged();
    } catch (err) {
      setTrustError(String(err));
    } finally {
      setTrustBusy(false);
    }
  };

  const selectedSession = sessions.find((s) => s.id === extractor.session_id);

  return (
    <div className="grid gap-4 border border-[var(--color-border)] p-3 bg-[var(--color-paper)]">
      {/* 1. Extraction Mode Selector */}
      <div className="flex flex-col gap-1 text-sm">
        <label className="font-bold text-xs uppercase tracking-wider text-[var(--color-ink)]">
          Extraction mode
        </label>
        <Select
          aria-label={`Source ${sourceIndex + 1} Extraction mode`}
          value={mode}
          options={MODE_OPTIONS}
          onChange={handleModeChange}
        />
      </div>

      {/* 2. Static & Browser Mode: Locators & Regex Transforms */}
      {(mode === "static" || mode === "browser") && (
        <div className="grid gap-3">
          <h5 className="font-semibold text-xs uppercase tracking-wider border-b border-[var(--color-border)] pb-1">
            Locators (CSS or XPath)
          </h5>

          {/* Required Locators */}
          <div className="grid gap-2">
            <label className="flex flex-col gap-1 text-xs">
              <span className="font-medium">Job container locator *</span>
              <Input
                aria-label={`Source ${sourceIndex + 1} Job container locator`}
                placeholder="e.g. div.job-card or xpath://article[@data-job-id]"
                value={extractor.job_container || ""}
                onChange={(e) =>
                  updateExtractor({ job_container: e.target.value || undefined })
                }
              />
            </label>

            <label className="flex flex-col gap-1 text-xs">
              <span className="font-medium">Title locator *</span>
              <Input
                aria-label={`Source ${sourceIndex + 1} Title locator`}
                placeholder="e.g. h3.title or xpath:.//h3"
                value={extractor.title_selector || ""}
                onChange={(e) =>
                  updateExtractor({ title_selector: e.target.value || undefined })
                }
              />
            </label>

            <label className="flex flex-col gap-1 text-xs">
              <span className="font-medium">Destination link locator *</span>
              <Input
                aria-label={`Source ${sourceIndex + 1} Destination link locator`}
                placeholder="e.g. a.apply-link or xpath:.//a"
                value={extractor.url_selector || ""}
                onChange={(e) =>
                  updateExtractor({ url_selector: e.target.value || undefined })
                }
              />
            </label>
          </div>

          {/* Optional Field Locators */}
          <details className="border border-[var(--color-border)] p-2 bg-[var(--color-surface)]">
            <summary className="cursor-pointer text-xs font-semibold uppercase select-none">
              Optional field locators
            </summary>
            <div className="grid gap-2 pt-2 text-xs">
              <label className="flex flex-col gap-1">
                Location locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Location locator`}
                  placeholder="e.g. span.location"
                  value={extractor.location_selector || ""}
                  onChange={(e) =>
                    updateExtractor({
                      location_selector: e.target.value || undefined,
                    })
                  }
                />
              </label>

              <label className="flex flex-col gap-1">
                Compensation locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Compensation locator`}
                  placeholder="e.g. span.salary"
                  value={extractor.salary_selector || ""}
                  onChange={(e) =>
                    updateExtractor({
                      salary_selector: e.target.value || undefined,
                    })
                  }
                />
              </label>

              <label className="flex flex-col gap-1">
                Description locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Description locator`}
                  placeholder="e.g. div.description"
                  value={extractor.description_selector || ""}
                  onChange={(e) =>
                    updateExtractor({
                      description_selector: e.target.value || undefined,
                    })
                  }
                />
              </label>

              <label className="flex flex-col gap-1">
                Posting date locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Posting date locator`}
                  placeholder="e.g. time.posted-date"
                  value={extractor.date_selector || ""}
                  onChange={(e) =>
                    updateExtractor({ date_selector: e.target.value || undefined })
                  }
                />
              </label>

              <label className="flex flex-col gap-1">
                Posting ID locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Posting ID locator`}
                  placeholder="e.g. span.job-id or xpath:.//@data-id"
                  value={extractor.id_selector || ""}
                  onChange={(e) =>
                    updateExtractor({ id_selector: e.target.value || undefined })
                  }
                />
              </label>

              <label className="flex flex-col gap-1">
                Employer logo locator
                <Input
                  aria-label={`Source ${sourceIndex + 1} Employer logo locator`}
                  placeholder="e.g. img.company-logo"
                  value={extractor.employer_logo_selector || ""}
                  onChange={(e) =>
                    updateExtractor({
                      employer_logo_selector: e.target.value || undefined,
                    })
                  }
                />
              </label>
            </div>
          </details>

          {/* Attributes and Regex Details */}
          <details className="border border-[var(--color-border)] p-2 bg-[var(--color-surface)]">
            <summary className="cursor-pointer text-xs font-semibold uppercase select-none">
              Attributes and Regex Transforms
            </summary>
            <div className="grid gap-2 pt-2 text-xs">
              <div className="grid grid-cols-2 gap-2">
                <label className="flex flex-col gap-1">
                  Title attribute
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Title attribute`}
                    placeholder="text (default)"
                    value={extractor.title_attribute || ""}
                    onChange={(e) =>
                      updateExtractor({
                        title_attribute: e.target.value || undefined,
                      })
                    }
                  />
                </label>
                <label className="flex flex-col gap-1">
                  Title regex
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Title regex`}
                    placeholder="(?P<value>.*)"
                    value={extractor.title_regex || ""}
                    onChange={(e) =>
                      updateExtractor({
                        title_regex: e.target.value || undefined,
                      })
                    }
                  />
                </label>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <label className="flex flex-col gap-1">
                  URL attribute
                  <Input
                    aria-label={`Source ${sourceIndex + 1} URL attribute`}
                    placeholder="href (default)"
                    value={extractor.url_attribute || ""}
                    onChange={(e) =>
                      updateExtractor({
                        url_attribute: e.target.value || undefined,
                      })
                    }
                  />
                </label>
                <label className="flex flex-col gap-1">
                  URL regex
                  <Input
                    aria-label={`Source ${sourceIndex + 1} URL regex`}
                    placeholder="(?P<value>.*)"
                    value={extractor.link_regex || ""}
                    onChange={(e) =>
                      updateExtractor({
                        link_regex: e.target.value || undefined,
                      })
                    }
                  />
                </label>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <label className="flex flex-col gap-1">
                  Location attribute
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Location attribute`}
                    placeholder="text"
                    value={extractor.location_attribute || ""}
                    onChange={(e) =>
                      updateExtractor({
                        location_attribute: e.target.value || undefined,
                      })
                    }
                  />
                </label>
                <label className="flex flex-col gap-1">
                  Location regex
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Location regex`}
                    placeholder="(?P<value>.*)"
                    value={extractor.location_regex || ""}
                    onChange={(e) =>
                      updateExtractor({
                        location_regex: e.target.value || undefined,
                      })
                    }
                  />
                </label>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <label className="flex flex-col gap-1">
                  Salary attribute
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Salary attribute`}
                    placeholder="text"
                    value={extractor.salary_attribute || ""}
                    onChange={(e) =>
                      updateExtractor({
                        salary_attribute: e.target.value || undefined,
                      })
                    }
                  />
                </label>
                <label className="flex flex-col gap-1">
                  Salary regex
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Salary regex`}
                    placeholder="(?P<value>.*)"
                    value={extractor.salary_regex || ""}
                    onChange={(e) =>
                      updateExtractor({
                        salary_regex: e.target.value || undefined,
                      })
                    }
                  />
                </label>
              </div>
            </div>
          </details>

          {/* Navigation & Dynamic Locators */}
          <div className="grid gap-2 border-t border-[var(--color-border)] pt-2">
            <h5 className="font-semibold text-xs uppercase tracking-wider">
              Pagination and Navigation
            </h5>

            <label className="flex flex-col gap-1 text-xs">
              Detail page link locator
              <Input
                aria-label={`Source ${sourceIndex + 1} Detail page link locator`}
                placeholder="e.g. a.details"
                value={extractor.detail_selector || ""}
                onChange={(e) =>
                  updateExtractor({ detail_selector: e.target.value || undefined })
                }
              />
            </label>

            <label className="flex flex-col gap-1 text-xs">
              Next page locator
              <Input
                aria-label={`Source ${sourceIndex + 1} Next page locator`}
                placeholder="e.g. a.pagination-next"
                value={extractor.next_page_selector || ""}
                onChange={(e) =>
                  updateExtractor({
                    next_page_selector: e.target.value || undefined,
                  })
                }
              />
            </label>

            {mode === "browser" && (
              <>
                <label className="flex flex-col gap-1 text-xs">
                  Load more locator
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Load more locator`}
                    placeholder="e.g. button.load-more"
                    value={extractor.load_more_selector || ""}
                    onChange={(e) =>
                      updateExtractor({
                        load_more_selector: e.target.value || undefined,
                      })
                    }
                  />
                </label>

                <label className="flex flex-col gap-1 text-xs">
                  Readiness wait locator
                  <Input
                    aria-label={`Source ${sourceIndex + 1} Readiness wait locator`}
                    placeholder="e.g. div.job-card"
                    value={extractor.wait_for_selector || ""}
                    onChange={(e) =>
                      updateExtractor({
                        wait_for_selector: e.target.value || undefined,
                      })
                    }
                  />
                </label>
              </>
            )}

            <label className="flex flex-col gap-1 text-xs">
              URL match pattern (glob)
              <Input
                aria-label={`Source ${sourceIndex + 1} URL match pattern`}
                placeholder="e.g. */jobs/*"
                value={extractor.url_pattern || ""}
                onChange={(e) =>
                  updateExtractor({ url_pattern: e.target.value || undefined })
                }
              />
            </label>
          </div>

          {/* Test Extractor Button and Preview */}
          <div className="pt-2 border-t border-[var(--color-border)]">
            <Button
              type="button"
              size="sm"
              aria-label={`Test extractor`}
              disabled={testing}
              onClick={handleTestExtractor}
            >
              {testing ? "Testing extractor…" : "Test extractor"}
            </Button>

            {testError && (
              <div
                role="alert"
                className="mt-2 text-xs text-[var(--color-danger)] border border-[var(--color-danger)] p-2 bg-[var(--color-paper)]"
              >
                {testError}
              </div>
            )}

            {testResult && (
              <div className="mt-3 grid gap-2 border border-[var(--color-border)] p-2.5 bg-[var(--color-surface)] text-xs">
                <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border)] pb-1.5">
                  <span className="font-bold uppercase tracking-wider">
                    Extraction test results ({testResult.samples.length} samples)
                  </span>
                  <span className="px-1.5 py-0.5 border border-[var(--color-border)] uppercase font-mono text-[10px]">
                    Termination: {testResult.termination}
                  </span>
                </div>

                {testResult.warnings.length > 0 && (
                  <div
                    role="alert"
                    className="border border-[var(--color-warning)] p-2 text-[var(--color-warning)] bg-[var(--color-paper)]"
                  >
                    {testResult.warnings.map((w, i) => (
                      <p key={i}>{w}</p>
                    ))}
                  </div>
                )}

                {testResult.field_match_counts && (
                  <div className="flex flex-wrap gap-2 text-[11px] text-[var(--color-ink-mute)]">
                    {Object.entries(testResult.field_match_counts).map(
                      ([field, count]) => (
                        <span key={field} className="border px-1 py-0.5">
                          {field}: {count}
                        </span>
                      )
                    )}
                  </div>
                )}

                <div className="grid gap-2 max-h-60 overflow-y-auto">
                  {testResult.samples.map((sample, idx) => (
                    <div
                      key={idx}
                      className="border border-[var(--color-border)] p-2 bg-[var(--color-paper)] grid gap-1"
                    >
                      <div className="font-semibold text-xs flex justify-between">
                        <span>{sample.title}</span>
                        {sample.salary && (
                          <span className="font-mono">{sample.salary}</span>
                        )}
                      </div>
                      <div className="flex flex-wrap gap-2 text-[11px] text-[var(--color-ink-mute)]">
                        {sample.location && <span>{sample.location}</span>}
                        {sample.date_posted && <span>{sample.date_posted}</span>}
                        {sample.external_id && (
                          <span>ID: {sample.external_id}</span>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 3. Browser Runtime & Authenticated Session Panel */}
      {mode === "browser" && (
        <div className="grid gap-3 border-t border-[var(--color-border)] pt-3">
          <h5 className="font-semibold text-xs uppercase tracking-wider">
            Browser Runtime & Session
          </h5>

          {/* Browser Runtime Indicator */}
          {browserRuntime && (
            <div
              className={`border p-2.5 text-xs ${
                browserRuntime.available
                  ? "border-[var(--color-border)] bg-[var(--color-paper-2)]"
                  : "border-[var(--color-warning)] text-[var(--color-warning)] bg-[var(--color-paper)]"
              }`}
            >
              {browserRuntime.available ? (
                <div>
                  <span className="font-bold uppercase tracking-wider text-[var(--color-success)]">
                    Chromium runtime detected
                  </span>
                  <p className="text-[11px] font-mono break-all mt-0.5">
                    {browserRuntime.executable_path} ({browserRuntime.version})
                  </p>
                </div>
              ) : (
                <div>
                  <span className="font-bold uppercase tracking-wider">
                    Browser runtime missing
                  </span>
                  <p className="mt-0.5">
                    {browserRuntime.message ||
                      "Install Google Chrome, Chromium, Brave, or Microsoft Edge to enable browser scraping."}
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Browser Bounds Inputs */}
          <div className="grid grid-cols-3 gap-2 text-xs">
            <label className="flex flex-col gap-1">
              Navigation timeout (s)
              <Input
                aria-label={`Source ${sourceIndex + 1} Navigation timeout`}
                type="number"
                value={extractor.navigation_timeout_s ?? 30}
                onChange={(e) =>
                  updateExtractor({
                    navigation_timeout_s: Number(e.target.value) || 30,
                  })
                }
              />
            </label>

            <label className="flex flex-col gap-1">
              Max pages
              <Input
                aria-label={`Source ${sourceIndex + 1} Max pages`}
                type="number"
                value={extractor.max_pages ?? 100}
                onChange={(e) =>
                  updateExtractor({
                    max_pages: Number(e.target.value) || 100,
                  })
                }
              />
            </label>

            <label className="flex flex-col gap-1">
              Max duration (s)
              <Input
                aria-label={`Source ${sourceIndex + 1} Max duration`}
                type="number"
                value={extractor.max_source_duration_s ?? 180}
                onChange={(e) =>
                  updateExtractor({
                    max_source_duration_s: Number(e.target.value) || 180,
                  })
                }
              />
            </label>
          </div>

          {/* Session Selector & Management */}
          <div className="grid gap-2 border border-[var(--color-border)] p-2.5 bg-[var(--color-surface)] text-xs">
            <div className="flex items-center justify-between gap-2">
              <label className="font-semibold uppercase">
                Isolated Login Session
              </label>
              {selectedSession && (
                <span
                  className={`px-1.5 py-0.5 text-[10px] font-mono uppercase border ${
                    selectedSession.status === "ready"
                      ? "border-[var(--color-success)] text-[var(--color-success)]"
                      : "border-[var(--color-warning)] text-[var(--color-warning)]"
                  }`}
                >
                  {selectedSession.status === "ready" ? "Ready" : "Needs login"}
                </span>
              )}
            </div>

            <Select
              aria-label={`Source ${sourceIndex + 1} Browser session`}
              value={extractor.session_id || ""}
              options={[
                { value: "", label: "None (unauthenticated)" },
                ...sessions.map((s) => ({
                  value: s.id,
                  label: `${s.label} (${s.domain}) · ${s.status}`,
                })),
              ]}
              onChange={(sessionId) =>
                updateExtractor({ session_id: sessionId || null })
              }
            />

            <div className="flex flex-wrap gap-2 pt-1">
              <Button
                type="button"
                size="sm"
                aria-label="Open login session"
                onClick={() => setShowOpenSession(!showOpenSession)}
                disabled={sessionBusy}
              >
                {showOpenSession ? "Close login form" : "Open login session"}
              </Button>

              <Button
                type="button"
                size="sm"
                aria-label="Save session"
                disabled={!extractor.session_id || sessionBusy}
                onClick={handleSaveSession}
              >
                Save session
              </Button>

              <Button
                type="button"
                size="sm"
                variant="danger"
                aria-label="Clear session"
                disabled={!extractor.session_id || sessionBusy}
                onClick={handleClearSession}
              >
                Clear session
              </Button>
            </div>

            {showOpenSession && (
              <div className="border border-[var(--color-border)] p-2.5 mt-2 bg-[var(--color-paper)] grid gap-2">
                <label className="flex flex-col gap-1">
                  Session label
                  <Input
                    aria-label="New session label"
                    value={openLabel}
                    onChange={(e) => setOpenLabel(e.target.value)}
                  />
                </label>

                <label className="flex flex-col gap-1">
                  Login URL
                  <Input
                    aria-label="New session login URL"
                    value={openUrl}
                    onChange={(e) => setOpenUrl(e.target.value)}
                  />
                </label>

                <div className="flex gap-2 pt-1">
                  <Button
                    type="button"
                    size="sm"
                    variant="inverted"
                    disabled={sessionBusy || !openUrl.trim()}
                    onClick={handleOpenSession}
                  >
                    Launch login browser
                  </Button>
                </div>
              </div>
            )}

            {sessionActionMsg && (
              <p className="text-[11px] text-[var(--color-ink-mute)] mt-1">
                {sessionActionMsg}
              </p>
            )}

            <p className="text-[11px] text-[var(--color-ink-mute)]">
              YAML exports store only the session reference ID, never credentials or cookies.
            </p>
          </div>
        </div>
      )}

      {/* 4. Custom Script & Trust Panel */}
      {mode === "script" && (
        <div className="grid gap-3 border-t border-[var(--color-border)] pt-3">
          <h5 className="font-semibold text-xs uppercase tracking-wider">
            Custom Script Configuration
          </h5>

          {/* Script Runtime Selector */}
          <div className="flex flex-col gap-1 text-xs">
            <label className="font-semibold uppercase">Script runtime</label>
            <Select
              aria-label={`Source ${sourceIndex + 1} Script runtime`}
              value={extractor.script?.runtime || "js"}
              options={RUNTIME_OPTIONS}
              onChange={(rt) =>
                updateExtractor({
                  script: {
                    runtime: rt as ScriptRuntime,
                    timeout_s: extractor.script?.timeout_s ?? 30,
                    interpreter: extractor.script?.interpreter || null,
                    code: extractor.script?.code || null,
                    path: extractor.script?.path || null,
                    args: extractor.script?.args || [],
                  },
                })
              }
            />
          </div>

          {/* Interpreter and Bounds */}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <label className="flex flex-col gap-1">
              Interpreter path (absolute)
              <Input
                aria-label={`Source ${sourceIndex + 1} Script interpreter`}
                placeholder="Absolute executable path, e.g. /usr/bin/python3"
                value={extractor.script?.interpreter || ""}
                onChange={(e) =>
                  updateExtractor({
                    script: {
                      runtime: extractor.script?.runtime || "js",
                      timeout_s: extractor.script?.timeout_s ?? 30,
                      interpreter: e.target.value || null,
                      code: extractor.script?.code || null,
                      path: extractor.script?.path || null,
                      args: extractor.script?.args || [],
                    },
                  })
                }
              />
            </label>

            <label className="flex flex-col gap-1">
              Timeout (s)
              <Input
                aria-label={`Source ${sourceIndex + 1} Script timeout`}
                type="number"
                value={extractor.script?.timeout_s ?? 30}
                onChange={(e) =>
                  updateExtractor({
                    script: {
                      runtime: extractor.script?.runtime || "js",
                      timeout_s: Number(e.target.value) || 30,
                      interpreter: extractor.script?.interpreter || null,
                      code: extractor.script?.code || null,
                      path: extractor.script?.path || null,
                      args: extractor.script?.args || [],
                    },
                  })
                }
              />
            </label>
          </div>

          {/* Arguments */}
          <label className="flex flex-col gap-1 text-xs">
            Arguments (space-separated)
            <Input
              aria-label={`Source ${sourceIndex + 1} Script arguments`}
              placeholder="e.g. --mode full --verbose"
              value={extractor.script?.args?.join(" ") || ""}
              onChange={(e) => {
                const args = e.target.value.trim()
                  ? e.target.value.trim().split(/\s+/)
                  : [];
                updateExtractor({
                  script: {
                    runtime: extractor.script?.runtime || "js",
                    timeout_s: extractor.script?.timeout_s ?? 30,
                    interpreter: extractor.script?.interpreter || null,
                    code: extractor.script?.code || null,
                    path: extractor.script?.path || null,
                    args,
                  },
                });
              }}
            />
          </label>

          {/* Script File Path and Inline Code */}
          <div className="grid gap-2 text-xs">
            <label className="flex flex-col gap-1">
              Script file path
              <Input
                aria-label={`Source ${sourceIndex + 1} Script file path`}
                placeholder="./scripts/custom-scraper.py"
                value={extractor.script?.path || ""}
                onChange={(e) =>
                  updateExtractor({
                    script: {
                      runtime: extractor.script?.runtime || "js",
                      timeout_s: extractor.script?.timeout_s ?? 30,
                      interpreter: extractor.script?.interpreter || null,
                      code: extractor.script?.code || null,
                      path: e.target.value || null,
                      args: extractor.script?.args || [],
                    },
                  })
                }
              />
            </label>

            <label className="flex flex-col gap-1">
              Inline script code (if no path specified)
              <textarea
                aria-label={`Source ${sourceIndex + 1} Inline script code`}
                className="font-mono text-xs border border-[var(--color-ink)] bg-[var(--color-paper)] p-2 min-h-28"
                placeholder="// Write NDJSON script code here"
                value={extractor.script?.code || ""}
                onChange={(e) =>
                  updateExtractor({
                    script: {
                      runtime: extractor.script?.runtime || "js",
                      timeout_s: extractor.script?.timeout_s ?? 30,
                      interpreter: extractor.script?.interpreter || null,
                      path: extractor.script?.path || null,
                      code: e.target.value || null,
                      args: extractor.script?.args || [],
                    },
                  })
                }
              />
            </label>
          </div>

          {/* Check Environment Button */}
          <div className="pt-1">
            <Button
              type="button"
              size="sm"
              aria-label="Check environment"
              disabled={envBusy}
              onClick={handleCheckEnvironment}
            >
              {envBusy ? "Checking…" : "Check environment"}
            </Button>

            {envError && (
              <p
                role="alert"
                className="text-xs text-[var(--color-danger)] mt-1"
              >
                {envError}
              </p>
            )}

            {envStatus && (
              <div className="mt-2 text-xs border border-[var(--color-border)] p-2 bg-[var(--color-paper-2)]">
                {envStatus.available ? (
                  <p className="text-[var(--color-success)] font-medium">
                    Environment ready: {envStatus.executable_path} (
                    {envStatus.version})
                  </p>
                ) : (
                  <p className="text-[var(--color-warning)] font-medium">
                    {envStatus.message || "Environment unavailable"}
                  </p>
                )}
                {envStatus.missing_modules &&
                  envStatus.missing_modules.length > 0 && (
                    <p className="text-[var(--color-warning)] mt-1">
                      Missing modules: {envStatus.missing_modules.join(", ")}
                    </p>
                  )}
              </div>
            )}
          </div>

          {/* Truthful Security & Privilege Disclosure */}
          <div className="border border-[var(--color-border)] p-3 text-xs bg-[var(--color-paper-2)] text-[var(--color-ink)]">
            <p>
              Custom scripts execute with your local user account privileges.
              Scrape enforces timeouts, structured arguments, and output
              validation, but does not provide OS sandboxing or guaranteed
              bot-challenge bypass.
            </p>
          </div>

          {trustSuccess && (
            <div className="border border-[var(--color-success)] p-2 text-xs text-[var(--color-success)] bg-[var(--color-paper)]">
              {trustSuccess}
            </div>
          )}

          {/* Trust Banner for Untrusted / Imported Scripts */}
          {(reviewSource?.needs_review ||
            !reviewSource?.script_trusted) && (
            <div
              role="alert"
              className="border border-[var(--color-warning)] p-3 grid gap-2 text-xs bg-[var(--color-paper)]"
            >
              <p className="font-semibold text-[var(--color-warning)]">
                This script configuration requires review and explicit trust
                before it can run or be scheduled.
              </p>

              {reviewSource?.id && reviewSource?.script_sha256 ? (
                <div className="flex flex-col gap-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      variant="inverted"
                      aria-label="Trust saved script"
                      disabled={trustBusy || scriptDirty}
                      onClick={handleTrustScript}
                    >
                      {trustBusy ? "Trusting…" : "Trust saved script"}
                    </Button>
                    {trustError && (
                      <span className="text-[var(--color-danger)]">
                        {trustError}
                      </span>
                    )}
                  </div>
                  {scriptDirty && (
                    <p className="text-[var(--color-ink-mute)]">
                      Save the workflow to fingerprint your changes, then trust it.
                    </p>
                  )}
                </div>
              ) : (
                <p className="text-[var(--color-ink-mute)]">
                  Save the workflow to fingerprint your changes, then trust it.
                </p>
              )}
            </div>
          )}

        </div>
      )}
    </div>
  );
}
