import { useState, useEffect, useMemo } from "react";
import type { Source } from "../../../bindings/Source";
import type { SourceKind } from "../../../bindings/SourceKind";
import type { SourceCadence } from "../../../bindings/SourceCadence";
import type { RoleType } from "../../../bindings/RoleType";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import { api, type DetectResult } from "../../../lib/api";
import { Modal } from "../../ui/Modal";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { Rule } from "../../ui/Rule";
import { ClockPicker } from "../../ui/ClockPicker";
import { notifyError, notifySuccess } from "../../../lib/toast";
import {
  validateWorkflowYaml,
  workflowSpecToYaml,
  type WorkflowSpec,
} from "../../../lib/yamlWorkflow";
import type { ClockScheduleState } from "../../../lib/cronClock";

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
  { value: "every_12h", label: "EVERY 12 HOURS" },
  { value: "every_6h", label: "EVERY 6 HOURS" },
  { value: "hourly", label: "HOURLY" },
  { value: "daily", label: "DAILY" },
  { value: "weekly", label: "WEEKLY" },
  { value: "manual", label: "MANUAL ONLY" },
  { value: "cron", label: "CUSTOM CRON..." },
];

const ALL_ROLE_TYPES: { id: RoleType; label: string }[] = [
  { id: "full_time", label: "FULL TIME" },
  { id: "contract", label: "CONTRACT" },
  { id: "internship", label: "INTERNSHIP" },
  { id: "part_time", label: "PART TIME" },
  { id: "seasonal", label: "SEASONAL" },
];

export function AddSourceModal({
  isOpen,
  onClose,
  onSourceSaved,
  initialSource,
}: AddSourceModalProps) {
  const isEditing = Boolean(initialSource);

  // Editor mode: form vs yaml
  const [editorMode, setEditorMode] = useState<"form" | "yaml">("form");
  const [yamlText, setYamlText] = useState("");

  // Core fields
  const [url, setUrl] = useState("");
  const [isDetecting, setIsDetecting] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<SourceKind>("greenhouse");
  const [identifier, setIdentifier] = useState("");

  // Schedule sub-mode
  const [scheduleSubTab, setScheduleSubTab] = useState<"presets" | "clock" | "cron">("presets");
  const [cadence, setCadence] = useState<SourceCadence>("global");
  const [cadenceCron, setCadenceCron] = useState("0 0 12 * * *");
  const [clockState, setClockState] = useState<ClockScheduleState>({
    hour12: 9,
    minute: 0,
    ampm: "AM",
    cadence: "daily",
  });
  const [cronValidation, setCronValidation] = useState<{
    ok: boolean;
    nextRuns: string[];
    error: string | null;
  } | null>(null);

  // Ingestion filters
  const [selectedRoleTypes, setSelectedRoleTypes] = useState<RoleType[]>([]);
  const [includeKeywords, setIncludeKeywords] = useState("");
  const [excludeKeywords, setExcludeKeywords] = useState("");
  const [locations, setLocations] = useState("");

  // Data Retention (TTL)
  const [retentionDays, setRetentionDays] = useState(30);
  const [retentionPolicy, setRetentionPolicy] = useState<"untracked_only" | "all">("untracked_only");

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

  // Live YAML Validation
  const liveValidation = useMemo(() => {
    if (editorMode !== "yaml") return { isValid: true, errors: [] };
    return validateWorkflowYaml(yamlText);
  }, [editorMode, yamlText]);

  // Populate state on open or initialSource change
  useEffect(() => {
    if (initialSource) {
      setName(initialSource.name);
      setKind(initialSource.kind);
      setIdentifier(initialSource.identifier);
      setUrl(initialSource.identifier);
      setCadence(initialSource.cadence || "global");
      setCadenceCron(initialSource.cadence_cron || "0 0 12 * * *");
      setRetentionDays(Number(initialSource.retention_days ?? 30));
      setRetentionPolicy(
        (initialSource.retention_policy as "untracked_only" | "all") || "untracked_only"
      );

      const cfg = (initialSource.config as Record<string, unknown>) || {};
      const sel = (cfg.selectors as Record<string, string>) || {};
      setSelectorItem(sel.item || (cfg.item as string) || ".job-row");
      setSelectorTitle(sel.title || (cfg.title as string) || ".title");
      setSelectorTitleRegex((cfg.title_regex as string) || "");
      setSelectorLink(sel.link || (cfg.link as string) || "a");
      setSelectorLinkRegex((cfg.link_regex as string) || "");
      setSelectorLocation(sel.location || (cfg.location as string) || ".location");
      setSelectorLocationRegex((cfg.location_regex as string) || "");
      setSelectorSalary(sel.salary || (cfg.salary as string) || "");
      setSelectorSalaryRegex((cfg.salary_regex as string) || "");

      const flt = (initialSource.filters as Record<string, unknown>) || {};
      if (Array.isArray(flt.role_types)) setSelectedRoleTypes(flt.role_types as RoleType[]);
      if (Array.isArray(flt.include_keywords)) setIncludeKeywords(flt.include_keywords.join(", "));
      if (Array.isArray(flt.exclude_keywords)) setExcludeKeywords(flt.exclude_keywords.join(", "));
      if (Array.isArray(flt.locations)) setLocations(flt.locations.join(", "));
    } else {
      setName("");
      setKind("greenhouse");
      setIdentifier("");
      setUrl("");
      setCadence("global");
      setCadenceCron("0 0 12 * * *");
      setRetentionDays(30);
      setRetentionPolicy("untracked_only");
      setSelectedRoleTypes([]);
      setIncludeKeywords("");
      setExcludeKeywords("");
      setLocations("");
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
    setScheduleSubTab("presets");
    setDryRunEvents(null);
  }, [initialSource, isOpen]);

  // Validate custom cron when cadence is 'cron'
  useEffect(() => {
    if (cadence !== "cron") {
      setCronValidation(null);
      return;
    }
    const timer = setTimeout(() => {
      api
        .validateCron(cadenceCron)
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
    const config: Record<string, unknown> = {
      url: url.trim() || identifier.trim(),
    };
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

  const assembleFilters = () => {
    const filters: Record<string, unknown> = {};
    if (selectedRoleTypes.length > 0) filters.role_types = selectedRoleTypes;
    if (includeKeywords.trim()) {
      filters.include_keywords = includeKeywords
        .split(",")
        .map((k) => k.trim())
        .filter(Boolean);
    }
    if (excludeKeywords.trim()) {
      filters.exclude_keywords = excludeKeywords
        .split(",")
        .map((k) => k.trim())
        .filter(Boolean);
    }
    if (locations.trim()) {
      filters.locations = locations
        .split(",")
        .map((l) => l.trim())
        .filter(Boolean);
    }
    return filters;
  };

  const handleSwitchToYaml = () => {
    const spec: WorkflowSpec = {
      version: 2,
      metadata: {
        name: name || "UNTITLED WORKFLOW",
        company: name || "UNTITLED WORKFLOW",
        enabled: true,
      },
      extractor: {
        kind,
        url: url || identifier || "https://example.com/careers",
        job_container: kind === "selector" ? selectorItem : undefined,
        title_selector: kind === "selector" ? selectorTitle : undefined,
        url_selector: kind === "selector" ? selectorLink : undefined,
        location_selector: kind === "selector" ? selectorLocation : undefined,
        salary_selector: kind === "selector" ? selectorSalary : undefined,
      },
      schedule: {
        cadence,
        cron: cadence === "cron" ? cadenceCron : undefined,
      },
      filters: {
        role_types: selectedRoleTypes.length > 0 ? selectedRoleTypes : undefined,
        include_keywords: includeKeywords
          ? includeKeywords.split(",").map((s) => s.trim()).filter(Boolean)
          : undefined,
        exclude_keywords: excludeKeywords
          ? excludeKeywords.split(",").map((s) => s.trim()).filter(Boolean)
          : undefined,
        locations: locations
          ? locations.split(",").map((s) => s.trim()).filter(Boolean)
          : undefined,
      },
      retention: {
        days: retentionDays,
        policy: retentionPolicy,
      },
    };

    setYamlText(workflowSpecToYaml(spec));
    setEditorMode("yaml");
  };

  const handleSwitchToForm = () => {
    const validation = validateWorkflowYaml(yamlText);
    if (!validation.isValid || !validation.parsedSpec) {
      const errs = validation.errors.map((e) => e.message).join("\n");
      notifyError(
        `Cannot switch to Form view due to YAML parsing issues:\n${errs}`,
        "YAML SYNTAX ERROR",
        5000
      );
      return;
    }

    const spec = validation.parsedSpec;
    setName(spec.metadata.name);
    setKind(spec.extractor.kind);
    setUrl(spec.extractor.url);
    setIdentifier(spec.extractor.url);

    if (spec.schedule?.cadence) setCadence(spec.schedule.cadence);
    if (spec.schedule?.cron) setCadenceCron(spec.schedule.cron);

    if (spec.retention?.days !== undefined) setRetentionDays(spec.retention.days);
    if (spec.retention?.policy) setRetentionPolicy(spec.retention.policy);

    if (spec.filters?.role_types) setSelectedRoleTypes(spec.filters.role_types);
    if (spec.filters?.include_keywords) setIncludeKeywords(spec.filters.include_keywords.join(", "));
    if (spec.filters?.exclude_keywords) setExcludeKeywords(spec.filters.exclude_keywords.join(", "));
    if (spec.filters?.locations) setLocations(spec.filters.locations.join(", "));

    setEditorMode("form");
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
      const validation = validateWorkflowYaml(yamlText);
      if (!validation.isValid || !validation.parsedSpec) {
        const errorDetails = validation.errors
          .map((e) => (e.line ? `Line ${e.line}, Col ${e.col}: ${e.message}` : e.message))
          .join("\n");
        notifyError(
          `YAML could not be saved due to parsing issues or invalid parameters:\n${errorDetails}`,
          "YAML PARSE ERROR",
          5000
        );
        return;
      }

      const spec = validation.parsedSpec;
      try {
        setIsSaving(true);
        const input = {
          name: spec.metadata.name,
          kind: spec.extractor.kind,
          identifier: spec.extractor.url,
          config: {
            url: spec.extractor.url,
            ...(spec.extractor.job_container ? { item: spec.extractor.job_container } : {}),
            ...(spec.extractor.title_selector ? { title: spec.extractor.title_selector } : {}),
            ...(spec.extractor.url_selector ? { link: spec.extractor.url_selector } : {}),
            ...(spec.extractor.location_selector ? { location: spec.extractor.location_selector } : {}),
            ...(spec.extractor.salary_selector ? { salary: spec.extractor.salary_selector } : {}),
          },
          cadence: spec.schedule?.cadence || "global",
          cadence_cron: spec.schedule?.cron || null,
          retention_days: BigInt(spec.retention?.days ?? 30),
          retention_policy: spec.retention?.policy || "untracked_only",
          filters: spec.filters || {},
        };

        if (isEditing && initialSource) {
          await api.updateSource({
            id: initialSource.id,
            ...input,
          });
        } else {
          await api.createSource(input);
        }

        notifySuccess("Workflow saved and activated successfully", "WORKFLOW SAVED");
        onSourceSaved();
        onClose();
      } catch (e) {
        notifyError(String(e), "SAVE FAILED");
      } finally {
        setIsSaving(false);
      }
      return;
    }

    // Form Mode Save
    if (!name.trim() || !identifier.trim()) {
      notifyError(
        "Workflow could not be saved: Name and target URL/identifier are required.",
        "VALIDATION ERROR",
        5000
      );
      return;
    }

    const config = assembleConfig();
    const filters = assembleFilters();

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
          retention_days: BigInt(retentionDays),
          retention_policy: retentionPolicy,
          filters,
        });
      } else {
        await api.createSource({
          name: name.trim(),
          kind,
          identifier: identifier.trim(),
          config,
          cadence,
          cadence_cron: cadence === "cron" ? cadenceCron.trim() : null,
          retention_days: BigInt(retentionDays),
          retention_policy: retentionPolicy,
          filters,
        });
      }
      notifySuccess("Workflow saved and activated successfully", "WORKFLOW SAVED");
      onSourceSaved();
      onClose();
    } catch (e) {
      notifyError(String(e), "SAVE FAILED");
    } finally {
      setIsSaving(false);
    }
  };

  const toggleRoleType = (rt: RoleType) => {
    setSelectedRoleTypes((prev) =>
      prev.includes(rt) ? prev.filter((r) => r !== rt) : [...prev, rt]
    );
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={isEditing ? `EDIT WORKFLOW: ${name.toUpperCase()}` : "CONFIGURE AUTONOMOUS WORKFLOW"}
    >
      <div className="flex flex-col gap-4 font-mono text-xs max-h-[82vh] overflow-y-auto pr-1">
        {/* Editor Mode Header */}
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

          {/* Live Validation Indicator in YAML mode */}
          {editorMode === "yaml" ? (
            <div>
              {liveValidation.isValid ? (
                <span className="px-2 py-0.5 border border-[var(--color-lime)] bg-[var(--color-lime)]/10 text-[var(--color-ink)] font-bold text-[10px]">
                  [✓ VALID SPEC · V2]
                </span>
              ) : (
                <span className="px-2 py-0.5 border border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)] font-bold text-[10px]">
                  [■ SYNTAX/PARAMETER ERROR]
                </span>
              )}
            </div>
          ) : (
            <span className="text-[10px] text-[var(--color-ink-mute)] uppercase">
              LINEAR DAG PIPELINE CONFIGURATION
            </span>
          )}
        </div>

        {editorMode === "yaml" ? (
          /* YAML Spec Editor Mode */
          <div className="flex flex-col gap-2">
            <div className="flex justify-between items-center text-[10px] text-[var(--color-ink-mute)]">
              <span>DECLARATIVE WORKFLOW DAG SPECIFICATION (VERSION: 2):</span>
              <button
                onClick={() => navigator.clipboard.writeText(yamlText)}
                className="px-2 py-0.5 border border-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer"
              >
                COPY YAML
              </button>
            </div>
            <textarea
              value={yamlText}
              onChange={(e) => setYamlText(e.target.value)}
              rows={20}
              className="w-full p-2.5 font-mono text-[11px] leading-relaxed border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-[var(--color-ink)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y"
              spellCheck={false}
            />

            {!liveValidation.isValid && (
              <div className="p-2 border border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)] font-bold text-[11px] flex flex-col gap-1">
                <span className="flex items-center gap-1.5">
                  <span>■</span> PARSE / SPECIFICATION ISSUES FOUND:
                </span>
                {liveValidation.errors.map((err, i) => (
                  <span key={i} className="font-mono text-[10px] font-normal">
                    · {err.line ? `Line ${err.line}, Col ${err.col}: ` : ""}
                    {err.message}
                  </span>
                ))}
              </div>
            )}
          </div>
        ) : (
          /* Form Mode */
          <>
            {/* Step 1: Extractor & Target */}
            <div className="flex flex-col gap-2">
              <Field label="1. EXTRACTOR & CAREERS TARGET URL">
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

            <div className="grid grid-cols-2 gap-3">
              <Field label="WORKFLOW NAME / COMPANY">
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Stripe, Acme Corp"
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
              label="TARGET IDENTIFIER / URL"
              hint="BOARD SLUG FOR ATS; FULL URL FOR JSON-LD & CUSTOM SELECTORS"
            >
              <Input
                variant="mono"
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                placeholder="e.g. stripe or https://acmecorp.com/careers"
              />
            </Field>

            <Rule />

            {/* Step 2: Dedicated Execution Schedule */}
            <div className="flex flex-col gap-2 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20">
              <div className="flex justify-between items-center border-b border-[var(--color-ink)] pb-1.5">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  2. WORKFLOW SCHEDULE
                </span>
                <div className="flex gap-1 text-[10px]">
                  <button
                    onClick={() => setScheduleSubTab("presets")}
                    className={`px-2 py-0.5 border ${
                      scheduleSubTab === "presets"
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)] font-bold"
                        : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                    }`}
                  >
                    PRESETS
                  </button>
                  <button
                    onClick={() => {
                      setScheduleSubTab("clock");
                      setCadence("cron");
                    }}
                    className={`px-2 py-0.5 border ${
                      scheduleSubTab === "clock"
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)] font-bold"
                        : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                    }`}
                  >
                    CLOCK
                  </button>
                  <button
                    onClick={() => {
                      setScheduleSubTab("cron");
                      setCadence("cron");
                    }}
                    className={`px-2 py-0.5 border ${
                      scheduleSubTab === "cron"
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)] font-bold"
                        : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                    }`}
                  >
                    CUSTOM CRON
                  </button>
                </div>
              </div>

              {scheduleSubTab === "presets" && (
                <div className="grid grid-cols-2 gap-3 pt-1">
                  <Field label="EXECUTION CADENCE" hint="ISOLATED RUN SCHEDULE">
                    <Select
                      value={cadence}
                      onChange={(e) => setCadence(e.target.value as SourceCadence)}
                      options={CADENCE_OPTIONS}
                    />
                  </Field>

                  <div className="flex flex-col justify-center text-[10px] text-[var(--color-ink-mute)]">
                    <span className="font-bold uppercase text-[var(--color-ink)]">
                      {cadence === "global"
                        ? "INHERITS MASTER SCRAPER SCHEDULE"
                        : cadence === "manual"
                        ? "RUNS ONLY WHEN EXPLICITLY TRIGGERED"
                        : "RUNS AUTOMATICALLY ON AN ISOLATED CADENCE"}
                    </span>
                    <span>Self-contained execution independent of other workflows.</span>
                  </div>
                </div>
              )}

              {scheduleSubTab === "clock" && (
                <div className="pt-2 flex flex-col items-center">
                  <ClockPicker
                    value={clockState}
                    onChange={(st, cron) => {
                      setClockState(st);
                      setCadenceCron(cron);
                    }}
                  />
                  <div className="mt-2 text-[10px] font-mono text-[var(--color-ink-mute)]">
                    GENERATED CRON: <span className="font-bold text-[var(--color-ink)]">{cadenceCron}</span>
                  </div>
                </div>
              )}

              {scheduleSubTab === "cron" && (
                <div className="pt-1 flex flex-col gap-2">
                  <Field label="6-FIELD CRON EXPRESSION" hint="SEC MIN HOUR DAY MONTH WEEKDAY">
                    <Input
                      variant="mono"
                      value={cadenceCron}
                      onChange={(e) => setCadenceCron(e.target.value)}
                      placeholder="0 0 12 * * *"
                    />
                  </Field>
                </div>
              )}

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
            </div>

            <Rule />

            {/* Step 3: Ingestion Filters */}
            <div className="flex flex-col gap-3 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20">
              <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                3. PER-WORKFLOW INGESTION FILTERS
              </span>

              {/* Role Types */}
              <div className="flex flex-col gap-1.5">
                <span className="text-[10px] font-bold text-[var(--color-ink-mute)] uppercase">
                  ALLOWED ROLE TYPES (LEAVE EMPTY FOR ALL):
                </span>
                <div className="flex flex-wrap gap-1.5">
                  {ALL_ROLE_TYPES.map((rt) => {
                    const active = selectedRoleTypes.includes(rt.id);
                    return (
                      <button
                        key={rt.id}
                        type="button"
                        onClick={() => toggleRoleType(rt.id)}
                        className={`px-2 py-0.5 text-xs font-mono border cursor-pointer ${
                          active
                            ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)] font-bold"
                            : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                        }`}
                      >
                        {active ? `[✓] ${rt.label}` : `[ ] ${rt.label}`}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <Field label="INCLUDE KEYWORDS" hint="COMMA-SEPARATED (MATCH ANY)">
                  <Input
                    value={includeKeywords}
                    onChange={(e) => setIncludeKeywords(e.target.value)}
                    placeholder="Software Engineer, Rust, Backend"
                  />
                </Field>

                <Field label="EXCLUDE KEYWORDS" hint="COMMA-SEPARATED (DISQUALIFY)">
                  <Input
                    value={excludeKeywords}
                    onChange={(e) => setExcludeKeywords(e.target.value)}
                    placeholder="Manager, Director, Staff"
                  />
                </Field>
              </div>

              <Field label="ALLOWED LOCATIONS" hint="COMMA-SEPARATED (MATCH ANY)">
                <Input
                  value={locations}
                  onChange={(e) => setLocations(e.target.value)}
                  placeholder="Remote, San Francisco, New York"
                />
              </Field>
            </div>

            <Rule />

            {/* Step 4: Data Retention Engine (TTL Policy) */}
            <div className="flex flex-col gap-3 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20">
              <div className="flex justify-between items-center">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  4. DATA RETENTION POLICY (TTL)
                </span>
                <span className="text-[10px] text-[var(--color-lime)] font-bold">
                  [★ SAFETY SHIELD ACTIVE]
                </span>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <Field
                  label="RETENTION LIFESPAN (DAYS)"
                  hint="0 = KEEP FOREVER; DEFAULT 30 DAYS"
                >
                  <Input
                    type="number"
                    min="0"
                    max="3650"
                    value={String(retentionDays)}
                    onChange={(e) => setRetentionDays(Math.max(0, parseInt(e.target.value) || 0))}
                  />
                </Field>

                <Field
                  label="RETENTION POLICY"
                  hint="POLICY REGARDING UNTRACKED POSTINGS"
                >
                  <Select
                    value={retentionPolicy}
                    onChange={(e) =>
                      setRetentionPolicy(e.target.value as "untracked_only" | "all")
                    }
                    options={[
                      {
                        value: "untracked_only",
                        label: "UNTRACKED ONLY (RECOMMENDED)",
                      },
                      {
                        value: "all",
                        label: "ALL UNTOUCHED (STRICT SAFETY)",
                      },
                    ]}
                  />
                </Field>
              </div>

              <div className="p-2 border border-[var(--color-ink)]/40 bg-[var(--color-paper)] text-[10px] text-[var(--color-ink-mute)] leading-relaxed">
                <strong className="text-[var(--color-ink)]">STRICT RETENTION SHIELD:</strong> The data retention engine
                will <strong>NEVER</strong> delete favorited postings (★), applications in progress
                (Applied, Interviewing, Offer, Rejected), or postings with candidate notes.
              </div>
            </div>

            {/* Step 5: Custom CSS Selectors for kind === 'selector' */}
            {kind === "selector" && (
              <>
                <Rule />
                <div className="flex flex-col gap-2 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
                  <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                    5. CSS SELECTORS & REGEX CAPTURE RULES
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
              </>
            )}

            {/* Step 6: Dry-Run Test Preview */}
            <div className="flex flex-col gap-2 pt-1">
              <div className="flex justify-between items-center">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                  {kind === "selector" ? "6. PRE-FLIGHT VERIFICATION" : "5. PRE-FLIGHT VERIFICATION"}
                </span>
                <Button
                  variant="default"
                  onClick={handleRunTransientDryRun}
                  disabled={isDryRunning || !identifier.trim()}
                >
                  {isDryRunning ? "TESTING WORKFLOW…" : "TEST WORKFLOW (DRY-RUN)"}
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
                        ZERO JOBS FOUND WITH CURRENT SPECIFICATION.
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
            disabled={isSaving}
          >
            {isSaving
              ? "SAVING…"
              : isEditing
              ? "UPDATE WORKFLOW"
              : "SAVE & ACTIVATE WORKFLOW"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
