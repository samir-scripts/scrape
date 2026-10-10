import { useState, useEffect } from "react";
import type { Source } from "../../../bindings/Source";
import type { SourceKind } from "../../../bindings/SourceKind";
import type { SourceCadence } from "../../../bindings/SourceCadence";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import { api, type DetectResult } from "../../../lib/api";
import { Modal } from "../../ui/Modal";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { Rule } from "../../ui/Rule";
import { sourceToYaml, yamlToSource } from "../../../lib/yamlJob";

interface AddSourceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSourceSaved: () => void;
  initialSource?: Source | null;
}

const KIND_OPTIONS = [
  { value: "greenhouse", label: "GREENHOUSE" },
  { value: "lever", label: "LEVER" },
  { value: "ashby", label: "ASHBY" },
  { value: "jsonld", label: "JSON-LD (SCHEMA.ORG)" },
  { value: "selector", label: "CUSTOM SCRAPER (CSS + REGEX)" },
];

const CADENCE_OPTIONS: { value: SourceCadence; label: string }[] = [
  { value: "global", label: "GLOBAL SCHEDULE (DEFAULT)" },
  { value: "hourly", label: "HOURLY" },
  { value: "every_6h", label: "EVERY 6 HOURS" },
  { value: "every_12h", label: "EVERY 12 HOURS" },
  { value: "daily", label: "DAILY (09:00)" },
  { value: "weekly", label: "WEEKLY (MON 09:00)" },
  { value: "manual", label: "MANUAL ONLY" },
  { value: "cron", label: "CUSTOM CRON..." },
];

export function AddSourceModal({
  isOpen,
  onClose,
  onSourceSaved,
  initialSource,
}: AddSourceModalProps) {
  const isEditing = Boolean(initialSource);

  // Form tab: 'form' vs 'yaml'
  const [editorMode, setEditorMode] = useState<"form" | "yaml">("form");
  const [yamlText, setYamlText] = useState("");
  const [yamlError, setYamlError] = useState<string | null>(null);

  // Core fields
  const [url, setUrl] = useState("");
  const [isDetecting, setIsDetecting] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<SourceKind>("greenhouse");
  const [identifier, setIdentifier] = useState("");
  const [cadence, setCadence] = useState<SourceCadence>("global");
  const [cadenceCron, setCadenceCron] = useState("0 0 12 * * *");
  const [cronValidation, setCronValidation] = useState<{
    ok: boolean;
    nextRuns: string[];
    error: string | null;
  } | null>(null);

  // Custom Selector fields
  const [selectorItem, setSelectorItem] = useState(".job-row");
  const [selectorTitle, setSelectorTitle] = useState(".title");
  const [selectorTitleRegex, setSelectorTitleRegex] = useState("");
  const [selectorLink, setSelectorLink] = useState("a");
  const [selectorLinkRegex, setSelectorLinkRegex] = useState("");
  const [selectorLocation, setSelectorLocation] = useState(".location");
  const [selectorLocationRegex, setSelectorLocationRegex] = useState("");
  const [selectorSalary, setSelectorSalary] = useState("");
  const [selectorSalaryRegex, setSelectorSalaryRegex] = useState("");

  // Dry-run testing
  const [isDryRunning, setIsDryRunning] = useState(false);
  const [dryRunEvents, setDryRunEvents] = useState<WorkerEvent[] | null>(null);

  const [isSaving, setIsSaving] = useState(false);

  // Populate state on open or initialSource change
  useEffect(() => {
    if (initialSource) {
      setName(initialSource.name);
      setKind(initialSource.kind);
      setIdentifier(initialSource.identifier);
      setUrl(initialSource.identifier);
      setCadence(initialSource.cadence || "global");
      setCadenceCron(initialSource.cadence_cron || "0 0 12 * * *");

      const cfg = (initialSource.config as any) || {};
      const sel = cfg.selectors || {};
      setSelectorItem(sel.item || cfg.item || ".job-row");
      setSelectorTitle(sel.title || cfg.title || ".title");
      setSelectorTitleRegex(cfg.title_regex || "");
      setSelectorLink(sel.link || cfg.link || "a");
      setSelectorLinkRegex(cfg.link_regex || "");
      setSelectorLocation(sel.location || cfg.location || ".location");
      setSelectorLocationRegex(cfg.location_regex || "");
      setSelectorSalary(sel.salary || cfg.salary || "");
      setSelectorSalaryRegex(cfg.salary_regex || "");
    } else {
      setName("");
      setKind("greenhouse");
      setIdentifier("");
      setUrl("");
      setCadence("global");
      setCadenceCron("0 0 12 * * *");
      setSelectorItem(".job-row");
      setSelectorTitle(".title");
      setSelectorTitleRegex("");
      setSelectorLink("a");
      setSelectorLinkRegex("");
      setSelectorLocation(".location");
      setSelectorLocationRegex("");
      setSelectorSalary("");
      setSelectorSalaryRegex("");
    }
    setEditorMode("form");
    setDryRunEvents(null);
    setYamlError(null);
  }, [initialSource, isOpen]);

  // Validate custom cron when cadence is 'cron'
  useEffect(() => {
    if (cadence !== "cron") {
      setCronValidation(null);
      return;
    }
    const timer = setTimeout(() => {
      api.validateCron(cadenceCron)
        .then((res) => {
          setCronValidation({
            ok: res.ok,
            nextRuns: res.next_runs || [],
            error: res.error || null,
          });
        })
        .catch((e) => {
          setCronValidation({
            ok: false,
            nextRuns: [],
            error: String(e),
          });
        });
    }, 250);
    return () => clearTimeout(timer);
  }, [cadence, cadenceCron]);

  const assembleConfig = () => {
    const config: Record<string, unknown> = {};
    if (kind === "selector") {
      config.selectors = {
        item: selectorItem.trim(),
        title: selectorTitle.trim(),
        link: selectorLink.trim(),
        ...(selectorLocation.trim() ? { location: selectorLocation.trim() } : {}),
        ...(selectorSalary.trim() ? { salary: selectorSalary.trim() } : {}),
      };
      if (selectorTitleRegex.trim()) config.title_regex = selectorTitleRegex.trim();
      if (selectorLinkRegex.trim()) config.link_regex = selectorLinkRegex.trim();
      if (selectorLocationRegex.trim()) config.location_regex = selectorLocationRegex.trim();
      if (selectorSalaryRegex.trim()) config.salary_regex = selectorSalaryRegex.trim();
    }
    return config;
  };

  const handleSwitchToYaml = () => {
    const yaml = sourceToYaml({
      name: name || "UNTITLED JOB",
      identifier: identifier || url || "https://example.com/careers",
      kind,
      cadence,
      cadence_cron: cadence === "cron" ? cadenceCron : null,
      config: assembleConfig(),
    });
    setYamlText(yaml);
    setYamlError(null);
    setEditorMode("yaml");
  };

  const handleSwitchToForm = () => {
    try {
      const parsed = yamlToSource(yamlText);
      setName(parsed.name);
      setIdentifier(parsed.identifier);
      setUrl(parsed.identifier);
      setKind(parsed.kind);
      setCadence(parsed.cadence);
      if (parsed.cadence_cron) setCadenceCron(parsed.cadence_cron);

      const sel = parsed.config?.selectors || {};
      if (sel.item) setSelectorItem(sel.item);
      if (sel.title) setSelectorTitle(sel.title);
      if (sel.link) setSelectorLink(sel.link);
      if (sel.location) setSelectorLocation(sel.location);
      if (sel.salary) setSelectorSalary(sel.salary);

      setSelectorTitleRegex(parsed.config?.title_regex || "");
      setSelectorLinkRegex(parsed.config?.link_regex || "");
      setSelectorLocationRegex(parsed.config?.location_regex || "");
      setSelectorSalaryRegex(parsed.config?.salary_regex || "");

      setYamlError(null);
      setEditorMode("form");
    } catch (e: any) {
      setYamlError(e.message || "Failed to parse YAML");
    }
  };

  const handleDetect = async () => {
    if (!url.trim()) return;
    try {
      setIsDetecting(true);
      const res: DetectResult = await api.detectSource(url.trim());
      setName(res.name);
      setKind(res.kind as SourceKind);
      setIdentifier(res.identifier);
    } catch (e) {
      console.error("Detection error:", e);
    } finally {
      setIsDetecting(false);
    }
  };

  const handleRunTransientDryRun = async () => {
    if (!identifier.trim()) return;
    try {
      setIsDryRunning(true);
      setDryRunEvents(null);
      const events = await api.testSourceTransient({
        name: name.trim() || "Preview",
        kind,
        identifier: identifier.trim(),
        config: assembleConfig() as any,
      });
      setDryRunEvents(events);
    } catch (e) {
      console.error("Dry run preview error:", e);
    } finally {
      setIsDryRunning(false);
    }
  };

  const handleSave = async () => {
    if (editorMode === "yaml") {
      try {
        const parsed = yamlToSource(yamlText);
        setName(parsed.name);
        setIdentifier(parsed.identifier);
        setKind(parsed.kind);
        setCadence(parsed.cadence);
        if (parsed.cadence_cron) setCadenceCron(parsed.cadence_cron);
      } catch (e: any) {
        setYamlError(e.message || "Invalid YAML");
        return;
      }
    }

    if (!name.trim() || !identifier.trim()) return;
    const config = assembleConfig();

    try {
      setIsSaving(true);
      if (isEditing && initialSource) {
        await api.updateSource({
          id: initialSource.id,
          name: name.trim(),
          identifier: identifier.trim(),
          config,
          cadence,
          cadence_cron: cadence === "cron" ? cadenceCron.trim() : null,
        });
      } else {
        await api.createSource({
          name: name.trim(),
          kind,
          identifier: identifier.trim(),
          config,
          cadence,
          cadence_cron: cadence === "cron" ? cadenceCron.trim() : null,
        });
      }
      onSourceSaved();
      onClose();
    } catch (e) {
      console.error("Failed to save scrape job:", e);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={isEditing ? `EDIT SCRAPE JOB: ${name.toUpperCase()}` : "ADD NEW SCRAPE JOB"}
    >
      <div className="flex flex-col gap-4 font-mono text-xs max-h-[80vh] overflow-y-auto pr-1">
        {/* Editor Mode Tabs */}
        <div className="flex items-center justify-between border-b border-[var(--color-ink)] pb-2">
          <div className="flex gap-2">
            <button
              onClick={() => {
                if (editorMode === "yaml") handleSwitchToForm();
              }}
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider cursor-pointer border ${
                editorMode === "form"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              [■ FORM VIEW]
            </button>
            <button
              onClick={() => {
                if (editorMode === "form") handleSwitchToYaml();
              }}
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider cursor-pointer border ${
                editorMode === "yaml"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              [ YAML SPEC ]
            </button>
          </div>
          <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">
            {editorMode === "form" ? "STEP-BY-STEP FORM" : "DECLARATIVE JOB SPEC"}
          </span>
        </div>

        {editorMode === "yaml" ? (
          /* YAML Spec Editor */
          <div className="flex flex-col gap-2">
            <div className="flex justify-between items-center text-[10px] text-[var(--color-ink-mute)]">
              <span>EDIT DECLARATIVE YAML DIRECTLY:</span>
              <button
                onClick={() => navigator.clipboard.writeText(yamlText)}
                className="px-2 py-0.5 border border-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer"
              >
                COPY YAML
              </button>
            </div>
            <textarea
              value={yamlText}
              onChange={(e) => {
                setYamlText(e.target.value);
                setYamlError(null);
              }}
              rows={16}
              className="w-full p-2.5 font-mono text-[11px] leading-relaxed border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-[var(--color-ink)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y"
              spellCheck={false}
            />
            {yamlError && (
              <div className="p-2 border border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)] font-bold text-[11px]">
                YAML SYNTAX ERROR: {yamlError}
              </div>
            )}
          </div>
        ) : (
          /* Standard Form Mode */
          <>
            {/* Step 1: URL Detection */}
            <div className="flex flex-col gap-2">
              <Field label="1. CAREERS PAGE OR TARGET URL">
                <div className="flex gap-2">
                  <Input
                    variant="mono"
                    value={url}
                    onChange={(e) => {
                      setUrl(e.target.value);
                      if (!isEditing && !identifier) setIdentifier(e.target.value);
                    }}
                    placeholder="https://boards.greenhouse.io/stripe or https://company.com/careers"
                    className="flex-1"
                  />
                  <Button
                    variant="default"
                    onClick={handleDetect}
                    disabled={isDetecting || !url.trim()}
                  >
                    {isDetecting ? "DETECTING…" : "AUTO-DETECT"}
                  </Button>
                </div>
              </Field>
            </div>

            <Rule />

            {/* Step 2: Source Details & Cadence */}
            <div className="grid grid-cols-2 gap-3">
              <Field label="COMPANY NAME">
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Acme Corp, Stripe"
                />
              </Field>

              <Field label="ADAPTER KIND">
                <Select
                  value={kind}
                  onChange={(e) => setKind(e.target.value as SourceKind)}
                  options={KIND_OPTIONS}
                />
              </Field>
            </div>

            <Field
              label="IDENTIFIER / URL TARGET"
              hint="BOARD SLUG FOR ATS; FULL URL FOR JSON-LD & CUSTOM SELECTOR"
            >
              <Input
                variant="mono"
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                placeholder="e.g. stripe or https://acmecorp.com/careers"
              />
            </Field>

            {/* Cadence Selection */}
            <div className="grid grid-cols-2 gap-3 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20">
              <Field label="EXECUTION CADENCE" hint="HOW OFTEN THIS JOB RUNS">
                <Select
                  value={cadence}
                  onChange={(e) => setCadence(e.target.value as SourceCadence)}
                  options={CADENCE_OPTIONS}
                />
              </Field>

              {cadence === "cron" ? (
                <Field label="6-FIELD CRON EXPRESSION" hint="SEC MIN HOUR DAY MONTH WEEKDAY">
                  <Input
                    variant="mono"
                    value={cadenceCron}
                    onChange={(e) => setCadenceCron(e.target.value)}
                    placeholder="0 0 */12 * * *"
                  />
                </Field>
              ) : (
                <div className="flex flex-col justify-center text-[10px] text-[var(--color-ink-mute)]">
                  <span className="font-bold uppercase text-[var(--color-ink)]">
                    {cadence === "global"
                      ? "INHERITS MASTER SCRAPER SCHEDULE"
                      : cadence === "manual"
                      ? "RUNS ONLY WHEN EXPLICITLY TRIGGERED"
                      : `RUNS AUTOMATICALLY ON AN ISOLATED CADENCE`}
                  </span>
                  <span>Independent execution without waiting for global batch.</span>
                </div>
              )}
            </div>

            {/* Cron validation preview */}
            {cadence === "cron" && cronValidation && (
              <div
                className={`p-2 border text-[10px] font-mono ${
                  cronValidation.ok
                    ? "border-[var(--color-lime)] bg-[var(--color-lime)]/10 text-[var(--color-ink)]"
                    : "border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)]"
                }`}
              >
                {cronValidation.ok ? (
                  <div>
                    <span className="font-bold text-[var(--color-ink)]">✓ VALID CRON EXPRESSION</span>
                    <div className="mt-1 flex flex-col gap-0.5 text-[var(--color-ink-mute)]">
                      <span>NEXT SCHEDULED RUNS:</span>
                      {cronValidation.nextRuns.map((r, i) => (
                        <span key={i} className="font-mono text-[var(--color-ink)]">
                          · {r}
                        </span>
                      ))}
                    </div>
                  </div>
                ) : (
                  <span>INVALID CRON: {cronValidation.error}</span>
                )}
              </div>
            )}

            {/* Custom CSS Selector & Regex Tuning */}
            {kind === "selector" && (
              <div className="flex flex-col gap-2 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  3. CSS SELECTORS & REGEX CAPTURE RULES
                </span>
                <div className="grid grid-cols-2 gap-2">
                  <Field label="ITEM SELECTOR (CONTAINER)">
                    <Input
                      variant="mono"
                      value={selectorItem}
                      onChange={(e) => setSelectorItem(e.target.value)}
                      placeholder=".job-row"
                    />
                  </Field>
                  <Field label="TITLE SELECTOR">
                    <Input
                      variant="mono"
                      value={selectorTitle}
                      onChange={(e) => setSelectorTitle(e.target.value)}
                      placeholder=".title"
                    />
                  </Field>
                  <Field label="TITLE REGEX (OPTIONAL)">
                    <Input
                      variant="mono"
                      value={selectorTitleRegex}
                      onChange={(e) => setSelectorTitleRegex(e.target.value)}
                      placeholder="e.g. Role:\s*(.*)"
                    />
                  </Field>
                  <Field label="LINK SELECTOR">
                    <Input
                      variant="mono"
                      value={selectorLink}
                      onChange={(e) => setSelectorLink(e.target.value)}
                      placeholder="a"
                    />
                  </Field>
                  <Field label="LINK REGEX (OPTIONAL)">
                    <Input
                      variant="mono"
                      value={selectorLinkRegex}
                      onChange={(e) => setSelectorLinkRegex(e.target.value)}
                      placeholder="e.g. (/jobs/\d+)"
                    />
                  </Field>
                  <Field label="LOCATION SELECTOR">
                    <Input
                      variant="mono"
                      value={selectorLocation}
                      onChange={(e) => setSelectorLocation(e.target.value)}
                      placeholder=".location"
                    />
                  </Field>
                  <Field label="LOCATION REGEX (OPTIONAL)">
                    <Input
                      variant="mono"
                      value={selectorLocationRegex}
                      onChange={(e) => setSelectorLocationRegex(e.target.value)}
                      placeholder="e.g. Location:\s*([^|]+)"
                    />
                  </Field>
                  <Field label="SALARY SELECTOR (OPTIONAL)">
                    <Input
                      variant="mono"
                      value={selectorSalary}
                      onChange={(e) => setSelectorSalary(e.target.value)}
                      placeholder=".salary"
                    />
                  </Field>
                  <Field label="SALARY REGEX (OPTIONAL)">
                    <Input
                      variant="mono"
                      value={selectorSalaryRegex}
                      onChange={(e) => setSelectorSalaryRegex(e.target.value)}
                      placeholder="e.g. (\$\d+[\d,]*\s*-\s*\$\d+[\d,]*)"
                    />
                  </Field>
                </div>
              </div>
            )}

            {/* Step 4: Dry-Run Test Preview */}
            <div className="flex flex-col gap-2 pt-1">
              <div className="flex justify-between items-center">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  4. PRE-FLIGHT VERIFICATION
                </span>
                <Button
                  variant="default"
                  onClick={handleRunTransientDryRun}
                  disabled={isDryRunning || !identifier.trim()}
                >
                  {isDryRunning ? "TESTING SCRAPER…" : "TEST SCRAPER (DRY-RUN)"}
                </Button>
              </div>

              {dryRunEvents && (
                <div className="p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 font-mono text-xs">
                  <div className="flex justify-between items-center mb-1 font-bold">
                    <span>
                      DRY-RUN RESULT:{" "}
                      {dryRunEvents.filter((e) => e.type === "job").length} JOBS EXTRACTED
                    </span>
                    <button
                      onClick={() => setDryRunEvents(null)}
                      className="text-[10px] hover:opacity-75 cursor-pointer"
                    >
                      [CLEAR]
                    </button>
                  </div>
                  <div className="max-h-36 overflow-y-auto flex flex-col gap-1 border border-[var(--color-ink)] bg-[var(--color-paper)] p-2">
                    {dryRunEvents.length === 0 ? (
                      <span className="text-[var(--color-ink-mute)]">
                        ZERO JOBS FOUND WITH CURRENT SELECTORS.
                      </span>
                    ) : (
                      dryRunEvents.map((ev, i) => (
                        <div key={i} className="text-[11px] truncate">
                          {ev.type === "job" ? (
                            <span>
                              <strong className="text-[var(--color-ink)]">{ev.title}</strong> ·{" "}
                              {ev.location || "REMOTE"} ({ev.role_type})
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
            </div>
          </>
        )}

        <Rule />

        {/* Modal Footer Actions */}
        <div className="flex justify-end gap-2 pt-1">
          <Button variant="ghost" onClick={onClose}>
            CANCEL
          </Button>
          <Button
            variant="inverted"
            onClick={handleSave}
            disabled={isSaving || !name.trim() || !identifier.trim()}
          >
            {isSaving
              ? "SAVING…"
              : isEditing
              ? "UPDATE SCRAPE JOB"
              : "SAVE & ACTIVATE JOB"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
