import { useState, useEffect, useMemo, useRef } from "react";
import type { Source } from "../../../bindings/Source";
import type { SourceKind } from "../../../bindings/SourceKind";
import type { SourceCadence } from "../../../bindings/SourceCadence";
import type { RoleType } from "../../../bindings/RoleType";
import type { WorkerEvent } from "../../../bindings/WorkerEvent";
import type { Settings } from "../../../bindings/Settings";
import { api, type DetectResult } from "../../../lib/api";
import { Modal } from "../../ui/Modal";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { Rule } from "../../ui/Rule";
import { Tooltip } from "../../ui/Tooltip";
import { ClockPicker } from "../../ui/ClockPicker";
import { notifyError, notifySuccess } from "../../../lib/toast";
import {
  validateWorkflowYaml,
  workflowSpecToYaml,
  canonicalizeAtsUrl,
  type WorkflowSpec,
} from "../../../lib/yamlWorkflow";
import {
  clockToCron,
  cronToClock,
  type ClockScheduleState,
} from "../../../lib/cronClock";

interface AddSourceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSourceSaved: () => void;
  initialSource?: Source | null;
  settings?: Settings;
}

const KIND_OPTIONS = [
  { value: "greenhouse", label: "GREENHOUSE" },
  { value: "lever", label: "LEVER" },
  { value: "ashby", label: "ASHBY" },
  { value: "jsonld", label: "JSON-LD (SCHEMA.ORG)" },
  { value: "selector", label: "CUSTOM SCRAPER (CSS + REGEX)" },
];

const SIMPLE_CADENCE_OPTIONS: { value: SourceCadence; label: string }[] = [
  { value: "daily", label: "DAILY (09:00 AM) [DEFAULT]" },
  { value: "every_12h", label: "EVERY 12 HOURS" },
  { value: "hourly", label: "HOURLY" },
  { value: "weekly", label: "WEEKLY" },
  { value: "manual", label: "MANUAL ONLY" },
  { value: "global", label: "GLOBAL SCHEDULE" },
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
  settings: propSettings,
}: AddSourceModalProps) {
  const isEditing = Boolean(initialSource);

  const [localSettings, setLocalSettings] = useState<Settings | null>(propSettings || null);

  useEffect(() => {
    if (propSettings) {
      setLocalSettings(propSettings);
    } else {
      api.getSettings().then(setLocalSettings).catch(() => {});
    }
  }, [propSettings]);

  const is24h = localSettings?.system_time_format === "24h";

  // Experience mode within modal: simple vs advanced
  const [modalMode, setModalMode] = useState<"simple" | "advanced">("simple");

  // Editor mode in advanced: form vs yaml
  const [editorMode, setEditorMode] = useState<"form" | "yaml">("form");
  const [yamlText, setYamlText] = useState("");

  // Hidden file input for YAML import
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Core fields
  const [url, setUrl] = useState("");
  const [isDetecting, setIsDetecting] = useState(false);
  const [detectedKind, setDetectedKind] = useState<SourceKind | null>(null);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<SourceKind>("greenhouse");
  const [identifier, setIdentifier] = useState("");

  // URL pattern globs
  const [urlPatterns, setUrlPatterns] = useState("");
  const [excludeUrlPatterns, setExcludeUrlPatterns] = useState("");

  // Schedule selection
  // Simple Mode: "presets" | "clock"
  const [simpleCadenceType, setSimpleCadenceType] = useState<"presets" | "clock">("presets");
  // Advanced Mode: "clock" | "cron"
  const [advCadenceType, setAdvCadenceType] = useState<"clock" | "cron">("clock");

  const [cadence, setCadence] = useState<SourceCadence>("daily");
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
  const [filterQuery, setFilterQuery] = useState("");
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
      const cfg = (initialSource.config as Record<string, unknown>) || {};
      const initialUrl = (cfg.url as string) || canonicalizeAtsUrl(initialSource.kind, initialSource.identifier);
      setUrl(initialUrl);
      setCadence(initialSource.cadence || "daily");
      setCadenceCron(initialSource.cadence_cron || "0 0 12 * * *");
      setRetentionDays(Number(initialSource.retention_days ?? 30));
      setRetentionPolicy(
        (initialSource.retention_policy as "untracked_only" | "all") || "untracked_only"
      );
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

      const pats: string[] = Array.isArray(cfg.url_patterns)
        ? (cfg.url_patterns as string[])
        : typeof cfg.url_pattern === "string"
        ? [cfg.url_pattern]
        : [];
      setUrlPatterns(pats.join(", "));
      const excPats: string[] = Array.isArray(cfg.exclude_url_patterns)
        ? (cfg.exclude_url_patterns as string[])
        : [];
      setExcludeUrlPatterns(excPats.join(", "));

      const flt = (initialSource.filters as Record<string, unknown>) || {};
      if (typeof flt.query === "string") setFilterQuery(flt.query);
      else setFilterQuery("");
      if (Array.isArray(flt.role_types)) setSelectedRoleTypes(flt.role_types as RoleType[]);
      if (Array.isArray(flt.include_keywords)) setIncludeKeywords(flt.include_keywords.join(", "));
      if (Array.isArray(flt.exclude_keywords)) setExcludeKeywords(flt.exclude_keywords.join(", "));
      if (Array.isArray(flt.locations)) setLocations(flt.locations.join(", "));

      // Schedule parsing
      if (initialSource.cadence === "cron" && initialSource.cadence_cron) {
        const parsed = cronToClock(initialSource.cadence_cron);
        if (parsed) {
          setClockState(parsed);
          setSimpleCadenceType("clock");
          setAdvCadenceType("clock");
        } else {
          setSimpleCadenceType("presets");
          setAdvCadenceType("cron");
        }
      } else {
        setSimpleCadenceType("presets");
        setAdvCadenceType("clock");
      }

      // Auto-elevation guard: If source has advanced features, open in advanced mode
      const hasAdvancedFeatures =
        initialSource.kind === "selector" ||
        initialSource.cadence === "cron" ||
        pats.length > 0 ||
        excPats.length > 0 ||
        Boolean(flt.query) ||
        (Array.isArray(flt.role_types) && flt.role_types.length > 0) ||
        (Array.isArray(flt.include_keywords) && flt.include_keywords.length > 0) ||
        (Array.isArray(flt.exclude_keywords) && flt.exclude_keywords.length > 0) ||
        (Array.isArray(flt.locations) && flt.locations.length > 0) ||
        Number(initialSource.retention_days) !== 30 ||
        initialSource.retention_policy !== "untracked_only";

      setModalMode(hasAdvancedFeatures ? "advanced" : "simple");
      setDetectedKind(initialSource.kind);
    } else {
      setName("");
      setKind("greenhouse");
      setIdentifier("");
      setUrl("");
      setCadence("daily");
      setCadenceCron("0 0 12 * * *");
      setSimpleCadenceType("presets");
      setAdvCadenceType("clock");
      setClockState({
        hour12: 9,
        minute: 0,
        ampm: "AM",
        cadence: "daily",
      });
      setUrlPatterns("");
      setExcludeUrlPatterns("");
      setRetentionDays(30);
      setRetentionPolicy("untracked_only");
      setSelectedRoleTypes([]);
      setFilterQuery("");
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
      setDetectedKind(null);
      setModalMode("simple");
    }
    setEditorMode("form");
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
    const incGlobs = urlPatterns
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    if (incGlobs.length > 0) {
      config.url_patterns = incGlobs;
      if (incGlobs.length === 1) config.url_pattern = incGlobs[0];
    }
    const excGlobs = excludeUrlPatterns
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    if (excGlobs.length > 0) {
      config.exclude_url_patterns = excGlobs;
    }
    return config;
  };

  const assembleFilters = () => {
    const filters: Record<string, unknown> = {};
    if (selectedRoleTypes.length > 0) filters.role_types = selectedRoleTypes;
    if (filterQuery.trim()) filters.query = filterQuery.trim();
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

  const applySpecToForm = (spec: WorkflowSpec) => {
    setName(spec.metadata.name || "");
    setKind(spec.extractor.kind || "greenhouse");
    setUrl(spec.extractor.url || "");

    let id = spec.extractor.url || "";
    if (spec.extractor.kind === "greenhouse") {
      const match = spec.extractor.url?.match(/boards\.greenhouse\.io\/([^/?#]+)/i);
      if (match) id = match[1];
    } else if (spec.extractor.kind === "lever") {
      const match = spec.extractor.url?.match(/jobs\.lever\.co\/([^/?#]+)/i);
      if (match) id = match[1];
    } else if (spec.extractor.kind === "ashby") {
      const match = spec.extractor.url?.match(/jobs\.ashbyhq\.com\/([^/?#]+)/i);
      if (match) id = match[1];
    }
    setIdentifier(id);

    if (spec.schedule?.cadence) setCadence(spec.schedule.cadence);
    if (spec.schedule?.cron) {
      setCadenceCron(spec.schedule.cron);
      const parsed = cronToClock(spec.schedule.cron);
      if (parsed) {
        setClockState(parsed);
        setAdvCadenceType("clock");
        setSimpleCadenceType("clock");
      } else {
        setAdvCadenceType("cron");
        setSimpleCadenceType("presets");
      }
    } else {
      setSimpleCadenceType("presets");
      setAdvCadenceType("clock");
    }

    if (spec.retention?.days !== undefined) setRetentionDays(spec.retention.days);
    if (spec.retention?.policy) setRetentionPolicy(spec.retention.policy);

    if (spec.filters?.role_types) setSelectedRoleTypes(spec.filters.role_types);
    if (spec.filters?.query) setFilterQuery(spec.filters.query);
    else setFilterQuery("");
    if (spec.filters?.include_keywords) setIncludeKeywords(spec.filters.include_keywords.join(", "));
    if (spec.filters?.exclude_keywords) setExcludeKeywords(spec.filters.exclude_keywords.join(", "));
    if (spec.filters?.locations) setLocations(spec.filters.locations.join(", "));

    if (spec.extractor.job_container) setSelectorItem(spec.extractor.job_container);
    if (spec.extractor.title_selector) setSelectorTitle(spec.extractor.title_selector);
    if (spec.extractor.url_selector) setSelectorLink(spec.extractor.url_selector);
    if (spec.extractor.location_selector) setSelectorLocation(spec.extractor.location_selector);
    if (spec.extractor.salary_selector) setSelectorSalary(spec.extractor.salary_selector);

    const pats: string[] = spec.extractor.url_patterns || (spec.extractor.url_pattern ? [spec.extractor.url_pattern] : []);
    setUrlPatterns(pats.join(", "));
    const excPats: string[] = spec.extractor.exclude_url_patterns || [];
    setExcludeUrlPatterns(excPats.join(", "));
  };

  const handleImportYamlText = (text: string) => {
    const validation = validateWorkflowYaml(text);
    if (!validation.isValid || !validation.parsedSpec) {
      const errorDetails = validation.errors
        .map((e) => (e.line ? `Line ${e.line}, Col ${e.col}: ${e.message}` : e.message))
        .join("\n");
      notifyError(
        `YAML could not be imported due to parsing issues or invalid parameters:\n${errorDetails}`,
        "YAML IMPORT ERROR",
        6000
      );
      return;
    }
    applySpecToForm(validation.parsedSpec);
    setYamlText(text);
    setModalMode("advanced");
    setEditorMode("yaml");
    notifySuccess("Workflow specification imported and validated successfully", "YAML IMPORTED");
  };

  const handleImportYamlFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (event) => {
      const text = event.target?.result as string;
      if (text) {
        handleImportYamlText(text);
      }
      if (fileInputRef.current) fileInputRef.current.value = "";
    };
    reader.readAsText(file);
  };

  const handleSwitchToYaml = () => {
    const incGlobs = urlPatterns
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const excGlobs = excludeUrlPatterns
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    const spec: WorkflowSpec = {
      version: 2,
      metadata: {
        name: name || "UNTITLED WORKFLOW",
        company: name || "UNTITLED WORKFLOW",
        enabled: true,
      },
      extractor: {
        kind,
        url: canonicalizeAtsUrl(kind, url || identifier) || "https://example.com/careers",
        job_container: kind === "selector" ? selectorItem : undefined,
        title_selector: kind === "selector" ? selectorTitle : undefined,
        url_selector: kind === "selector" ? selectorLink : undefined,
        location_selector: kind === "selector" ? selectorLocation : undefined,
        salary_selector: kind === "selector" ? selectorSalary : undefined,
        url_patterns: incGlobs.length > 0 ? incGlobs : undefined,
        exclude_url_patterns: excGlobs.length > 0 ? excGlobs : undefined,
      },
      schedule: {
        cadence,
        cron: cadence === "cron" ? cadenceCron : undefined,
      },
      filters: {
        role_types: selectedRoleTypes.length > 0 ? selectedRoleTypes : undefined,
        query: filterQuery.trim() ? filterQuery.trim() : undefined,
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

  const handleSwitchToForm = (forceDiscard = false) => {
    if (forceDiscard) {
      setEditorMode("form");
      return;
    }

    const validation = validateWorkflowYaml(yamlText);
    if (validation.parsedSpec) {
      applySpecToForm(validation.parsedSpec);
      setEditorMode("form");
      if (!validation.isValid) {
        const errs = validation.errors.map((e) => e.message).join("\n");
        notifyError(
          `Switched to Form view with warnings:\n${errs}`,
          "SPECIFICATION WARNING",
          5000
        );
        api.createNotification({
          kind: "error",
          title: "PARSE / SPECIFICATION ISSUE",
          message: `Specification has parameter warnings:\n${errs}`,
          action_label: "VIEW WORKFLOW",
          action_payload: name || null,
        }).catch(() => {});
      }
      return;
    }

    const errs = validation.errors.map((e) => e.message).join("\n");
    notifyError(
      `Cannot parse YAML syntax:\n${errs}\nClick 'DISCARD & RETURN TO FORM' to abort changes.`,
      "YAML SYNTAX ERROR",
      6000
    );
    api.createNotification({
      kind: "error",
      title: "PARSE / SPECIFICATION ISSUE",
      message: `Cannot parse YAML syntax:\n${errs}`,
      action_label: "VIEW WORKFLOW",
      action_payload: name || null,
    }).catch(() => {});
  };

  const handleDetect = async (targetUrl?: string) => {
    const candidate = targetUrl || url;
    if (!candidate.trim()) return;
    try {
      setIsDetecting(true);
      const res: DetectResult = await api.detectSource(candidate.trim());
      if (!name) setName(res.name);
      setKind(res.kind as SourceKind);
      setDetectedKind(res.kind as SourceKind);
      setIdentifier(res.identifier);
    } catch (e) {
      console.error("Detection error:", e);
    } finally {
      setIsDetecting(false);
    }
  };

  const handleRunTransientDryRun = async () => {
    if (!identifier.trim() && !url.trim()) return;
    try {
      setIsDryRunning(true);
      setDryRunEvents(null);
      const events = await api.testSourceTransient({
        name: name.trim() || "Preview",
        kind,
        identifier: identifier.trim() || url.trim(),
        config: assembleConfig() as any,
        filters: assembleFilters() as any,
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
            ...(spec.extractor.url_patterns ? { url_patterns: spec.extractor.url_patterns } : {}),
            ...(spec.extractor.url_pattern ? { url_pattern: spec.extractor.url_pattern } : {}),
            ...(spec.extractor.exclude_url_patterns ? { exclude_url_patterns: spec.extractor.exclude_url_patterns } : {}),
          },
          cadence: spec.schedule?.cadence || "daily",
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

    // Form Mode Save (Simple or Advanced)
    const effectiveTarget = identifier.trim() || url.trim();
    if (!name.trim() || !effectiveTarget) {
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
          identifier: effectiveTarget,
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
          identifier: effectiveTarget,
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

  const modalWidth = modalMode === "simple" ? "max-w-xl" : "max-w-5xl";

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={isEditing ? `EDIT WORKFLOW: ${name.toUpperCase()}` : "CONFIGURE WORKFLOW"}
      width={modalWidth}
    >
      <div className="flex flex-col gap-4 font-mono text-xs pr-1">
        {/* Top Header: Simple vs Advanced Mode Switcher & YAML Import / Validation */}
        <div className="flex items-center justify-between border-b border-[var(--color-ink)] pb-2 select-none">
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-1 border border-[var(--color-ink)] p-0.5 bg-[var(--color-paper-2)]/30">
              <Tooltip content="Simple Mode: Rapid setup with automated ATS detection and defaults">
                <button
                  type="button"
                  onClick={() => {
                    setModalMode("simple");
                    setEditorMode("form");
                  }}
                  className={`px-2.5 py-0.5 text-xs font-mono font-bold uppercase cursor-pointer ${
                    modalMode === "simple"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                      : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  [ SIMPLE ]
                </button>
              </Tooltip>
              <Tooltip content="Advanced Mode: custom selectors, clock/cron schedule, keywords, and YAML spec">
                <button
                  type="button"
                  onClick={() => setModalMode("advanced")}
                  className={`px-2.5 py-0.5 text-xs font-mono font-bold uppercase cursor-pointer ${
                    modalMode === "advanced"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                      : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  [ ADVANCED ]
                </button>
              </Tooltip>
            </div>

            <input
              ref={fileInputRef}
              type="file"
              accept=".yaml,.yml"
              className="hidden"
              onChange={handleImportYamlFile}
            />
            <Tooltip content="Import workflow specification (.yaml)">
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                aria-label="Import YAML"
                className="p-1 text-xs font-mono font-bold uppercase cursor-pointer border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] text-[var(--color-ink)] flex items-center justify-center transition-colors"
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="square"
                  strokeLinejoin="miter"
                >
                  <path d="M2 11v3h12v-3" />
                  <path d="M8 2v8" />
                  <path d="M4 6l4-4 4 4" />
                </svg>
              </button>
            </Tooltip>
          </div>

          {/* Right Header Status in Advanced Mode */}
          {modalMode === "advanced" && (
            <div className="flex items-center gap-2">
              <div className="flex gap-1">
                <button
                  type="button"
                  onClick={() => {
                    if (editorMode === "yaml") handleSwitchToForm();
                  }}
                  className={`px-2 py-0.5 text-[10px] font-bold uppercase cursor-pointer border ${
                    editorMode === "form"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                      : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  FORM
                </button>
                <button
                  type="button"
                  onClick={() => {
                    if (editorMode === "form") handleSwitchToYaml();
                  }}
                  className={`px-2 py-0.5 text-[10px] font-bold uppercase cursor-pointer border ${
                    editorMode === "yaml"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                      : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  YAML SPEC
                </button>
              </div>

              {editorMode === "yaml" && (
                <div>
                  {liveValidation.isValid ? (
                    <span className="px-1.5 py-0.5 border border-[var(--color-lime)] bg-[var(--color-lime)]/10 text-[var(--color-ink)] font-bold text-[10px]">
                      [✓ VALID SPEC]
                    </span>
                  ) : (
                    <span className="px-1.5 py-0.5 border border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)] font-bold text-[10px]">
                      [■ SYNTAX ERROR]
                    </span>
                  )}
                </div>
              )}
            </div>
          )}
        </div>


        {/* MODE 1: SIMPLE MODE (2 inputs, streamlined, zero flavor text) */}
        {modalMode === "simple" && (
          <div className="flex flex-col gap-4 py-1">
            <Field label="CAREERS TARGET URL / ATS BOARD">
              <div className="flex gap-2">
                <Input
                  variant="mono"
                  value={url}
                  onChange={(e) => {
                    setUrl(e.target.value);
                    if (!isEditing && !identifier) setIdentifier(e.target.value);
                  }}
                  onBlur={() => {
                    if (url.trim() && !detectedKind) handleDetect(url);
                  }}
                  placeholder="https://boards.greenhouse.io/stripe or https://company.com/careers"
                  className="flex-1"
                />
                <Button
                  variant="default"
                  onClick={() => handleDetect(url)}
                  disabled={isDetecting || !url.trim()}
                >
                  {isDetecting ? "DETECTING…" : "AUTO-DETECT"}
                </Button>
              </div>
            </Field>

            {detectedKind && (
              <div className="flex items-center gap-2 text-[10px]">
                <span className="px-1.5 py-0.5 border border-[var(--color-lime)] bg-[var(--color-lime)]/20 text-[var(--color-ink)] font-bold">
                  ✓ {detectedKind.toUpperCase()} ADAPTER DETECTED
                </span>
                <span className="text-[var(--color-ink-mute)]">
                  Using optimized schema ingestion
                </span>
              </div>
            )}

            <div className="flex flex-col gap-3">
              <Field label="COMPANY / WORKFLOW NAME">
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Stripe, Acme Corp"
                />
              </Field>

              {/* Execution Cadence: PRESETS or CLOCK (mutually exclusive) */}
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] font-mono font-bold uppercase text-[var(--color-ink)]">
                    EXECUTION CADENCE
                  </span>
                  <div className="flex gap-1 text-[10px]">
                    <button
                      type="button"
                      onClick={() => {
                        setSimpleCadenceType("presets");
                        if (cadence === "cron") setCadence("daily");
                      }}
                      className={`px-2 py-0.5 border cursor-pointer font-bold ${
                        simpleCadenceType === "presets"
                          ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                          : "border-[var(--color-ink)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                      }`}
                    >
                      PRESETS
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setSimpleCadenceType("clock");
                        setCadence("cron");
                        const dailyCron = clockToCron(
                          clockState.hour12,
                          clockState.minute,
                          clockState.ampm,
                          "daily"
                        );
                        setCadenceCron(dailyCron);
                      }}
                      className={`px-2 py-0.5 border cursor-pointer font-bold ${
                        simpleCadenceType === "clock"
                          ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                          : "border-[var(--color-ink)] text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                      }`}
                    >
                      CLOCK
                    </button>
                  </div>
                </div>

                {simpleCadenceType === "presets" ? (
                  <Select
                    value={cadence === "cron" ? "daily" : cadence}
                    onChange={(e) => setCadence(e.target.value as SourceCadence)}
                    options={SIMPLE_CADENCE_OPTIONS}
                  />
                ) : (
                  <div className="p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 flex flex-col items-center">
                    <ClockPicker
                      value={{ ...clockState, cadence: "daily" }}
                      onChange={(st, cron) => {
                        setClockState({ ...st, cadence: "daily" });
                        setCadenceCron(cron);
                        setCadence("cron");
                      }}
                      hideCadenceOptions={true}
                      is24h={is24h}
                    />
                    <div className="mt-2 text-[10px] font-mono text-[var(--color-ink-mute)]">
                      RUNS EVERY DAY AT{" "}
                      <strong className="text-[var(--color-ink)]">
                        {is24h
                          ? `${String((clockState.hour12 % 12) + (clockState.ampm === "PM" ? 12 : 0)).padStart(2, "0")}:${String(clockState.minute).padStart(2, "0")}`
                          : `${String(clockState.hour12).padStart(2, "0")}:${String(clockState.minute).padStart(2, "0")} ${clockState.ampm}`}
                      </strong>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <Rule />

            {/* Smart Defaults Summary Micro-Badge */}
            <div className="flex items-center justify-between p-2 border border-[var(--color-ink)]/30 bg-[var(--color-paper-2)]/30 text-[10px]">
              <span className="text-[var(--color-ink)] font-bold uppercase">
                DEFAULT POLICY: DAILY @ 09:00 AM · 30-DAY TTL · ALL ROLES
              </span>
              <Tooltip content="To customize data retention lifespan, custom selectors, or ingestion filters, switch to Advanced Mode above.">
                <span className="text-[var(--color-ink-mute)] underline cursor-help">
                  CUSTOMIZE IN ADVANCED →
                </span>
              </Tooltip>
            </div>

            {/* Action Bar */}
            <div className="flex justify-end gap-2 pt-2">
              <Button variant="default" onClick={onClose} disabled={isSaving}>
                CANCEL
              </Button>
              <Button variant="inverted" onClick={handleSave} disabled={isSaving || !name.trim() || !url.trim()}>
                {isSaving ? "SAVING…" : isEditing ? "UPDATE WORKFLOW" : "SAVE WORKFLOW"}
              </Button>
            </div>
          </div>
        )}

        {/* MODE 2: ADVANCED MODE */}
        {modalMode === "advanced" && editorMode === "yaml" && (
          <div className="flex flex-col gap-2">
            <div className="flex justify-between items-center text-[10px] text-[var(--color-ink-mute)]">
              <span>DECLARATIVE WORKFLOW SPECIFICATION (V2):</span>
              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => handleSwitchToForm(true)}
                  className="px-2 py-0.5 border border-[var(--color-cherry)] text-[var(--color-cherry)] hover:bg-[var(--color-cherry)] hover:text-white cursor-pointer font-bold"
                >
                  DISCARD & RETURN TO FORM
                </button>
                <button
                  type="button"
                  onClick={() => navigator.clipboard.writeText(yamlText)}
                  className="px-2 py-0.5 border border-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer"
                >
                  COPY YAML
                </button>
              </div>
            </div>
            <textarea
              value={yamlText}
              onChange={(e) => setYamlText(e.target.value)}
              rows={18}
              className="w-full p-2.5 font-mono text-[11px] leading-relaxed border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 text-[var(--color-ink)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] resize-y"
              spellCheck={false}
            />

            {!liveValidation.isValid && (
              <div className="p-2 border border-[var(--color-cherry)] bg-[var(--color-cherry)]/10 text-[var(--color-cherry)] font-bold text-[11px] flex flex-col gap-1">
                <span>■ PARSE / SPECIFICATION ISSUES:</span>
                {liveValidation.errors.map((err, i) => (
                  <span key={i} className="font-mono text-[10px] font-normal">
                    · {err.line ? `Line ${err.line}, Col ${err.col}: ` : ""}{err.message}
                  </span>
                ))}
              </div>
            )}

            <div className="flex justify-end gap-2 pt-2">
              <Button variant="default" onClick={onClose} disabled={isSaving}>
                CANCEL
              </Button>
              <Button variant="inverted" onClick={handleSave} disabled={isSaving}>
                {isSaving ? "SAVING…" : "SAVE WORKFLOW"}
              </Button>
            </div>
          </div>
        )}

        {modalMode === "advanced" && editorMode === "form" && (
          <div className="grid grid-cols-12 gap-4">
            {/* Left Column (7 cols): Full Configuration */}
            <div className="col-span-7 flex flex-col gap-3">
              {/* Target & Extractor */}
              <div className="flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)]">
                <Field label="EXTRACTOR & CAREERS URL">
                  <div className="flex gap-2">
                    <Input
                      variant="mono"
                      value={url}
                      onChange={(e) => {
                        setUrl(e.target.value);
                        if (!isEditing && !identifier) setIdentifier(e.target.value);
                      }}
                      placeholder="https://boards.greenhouse.io/stripe or https://company.com/careers"
                      className="flex-1 text-xs"
                    />
                    <Button
                      variant="default"
                      onClick={() => handleDetect(url)}
                      disabled={isDetecting || !url.trim()}
                    >
                      {isDetecting ? "…" : "DETECT"}
                    </Button>
                  </div>
                </Field>

                <div className="grid grid-cols-2 gap-2">
                  <Field label="WORKFLOW NAME">
                    <Input
                      value={name}
                      onChange={(e) => setName(e.target.value)}
                      placeholder="e.g. Stripe"
                      className="text-xs"
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

                <div className="grid grid-cols-2 gap-2 pt-2 border-t border-[var(--color-ink)]/20">
                  <Field label="INCLUDE URL GLOBS (MATCH)">
                    <Input
                      variant="mono"
                      value={urlPatterns}
                      onChange={(e) => setUrlPatterns(e.target.value)}
                      placeholder="e.g. */careers/*, example/link/*"
                      className="text-xs"
                    />
                  </Field>
                  <Field label="EXCLUDE URL GLOBS (DISCARD)">
                    <Input
                      variant="mono"
                      value={excludeUrlPatterns}
                      onChange={(e) => setExcludeUrlPatterns(e.target.value)}
                      placeholder="e.g. */archive/*, */login*"
                      className="text-xs"
                    />
                  </Field>
                </div>
              </div>

              {/* Execution Schedule: CLOCK SCHEDULE or CUSTOM CRON */}
              <div className="flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)]">
                <div className="flex justify-between items-center border-b border-[var(--color-ink)]/30 pb-1">
                  <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                    WORKFLOW SCHEDULE
                  </span>
                  <div className="flex gap-1 text-[10px]">
                    <button
                      type="button"
                      onClick={() => {
                        setAdvCadenceType("clock");
                        setCadence("cron");
                        const cronStr = clockToCron(
                          clockState.hour12,
                          clockState.minute,
                          clockState.ampm,
                          clockState.cadence
                        );
                        setCadenceCron(cronStr);
                      }}
                      className={`px-2 py-0.5 border cursor-pointer font-bold ${
                        advCadenceType === "clock"
                          ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                          : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                      }`}
                    >
                      CLOCK SCHEDULE
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setAdvCadenceType("cron");
                        setCadence("cron");
                      }}
                      className={`px-2 py-0.5 border cursor-pointer font-bold ${
                        advCadenceType === "cron"
                          ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                          : "border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                      }`}
                    >
                      CUSTOM CRON
                    </button>
                  </div>
                </div>

                {advCadenceType === "clock" ? (
                  <div className="py-1 flex flex-col items-center">
                    <ClockPicker
                      value={clockState}
                      onChange={(st, cron) => {
                        setClockState(st);
                        setCadenceCron(cron);
                        setCadence("cron");
                      }}
                      is24h={is24h}
                    />
                    <div className="mt-1 text-[10px] font-mono text-[var(--color-ink-mute)]">
                      CRON: <strong className="text-[var(--color-ink)]">{cadenceCron}</strong>
                    </div>
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5">
                    <Field label="CRON [TOKIO 6-FIELD]">
                      <Input
                        variant="mono"
                        value={cadenceCron}
                        onChange={(e) => {
                          setCadenceCron(e.target.value);
                          setCadence("cron");
                        }}
                        placeholder="0 0 12 * * *"
                        className="text-xs"
                      />
                    </Field>
                    {cronValidation && (
                      <span className={`text-[10px] ${cronValidation.ok ? "text-[var(--color-lime)]" : "text-[var(--color-cherry)]"}`}>
                        {cronValidation.ok ? `✓ VALID CRON (${cronValidation.nextRuns[0] || "SCHEDULED"})` : `ERR: ${cronValidation.error}`}
                      </span>
                    )}
                  </div>
                )}
              </div>

              {/* Ingestion Filters */}
              <div className="flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)]">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)] border-b border-[var(--color-ink)]/30 pb-1">
                  INGESTION FILTERS
                </span>

                <div className="flex flex-wrap gap-1">
                  {ALL_ROLE_TYPES.map((rt) => {
                    const active = selectedRoleTypes.includes(rt.id);
                    return (
                      <button
                        key={rt.id}
                        type="button"
                        onClick={() => toggleRoleType(rt.id)}
                        className={`px-1.5 py-0.5 text-[10px] font-mono border cursor-pointer ${
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

                <div className="grid grid-cols-2 gap-2 pt-1">
                  <Field label="INCLUDE KEYWORDS">
                    <Input
                      value={includeKeywords}
                      onChange={(e) => setIncludeKeywords(e.target.value)}
                      placeholder="e.g. Rust, Distributed"
                      className="text-xs"
                    />
                  </Field>
                  <Field label="EXCLUDE KEYWORDS">
                    <Input
                      value={excludeKeywords}
                      onChange={(e) => setExcludeKeywords(e.target.value)}
                      placeholder="e.g. Manager, Sales"
                      className="text-xs"
                    />
                  </Field>
                </div>

                <div className="pt-1">
                  <Field label="CUSTOM FILTER POLICY (SQL LIKE / BOOLEAN QUERY)">
                    <Input
                      value={filterQuery}
                      onChange={(e) => setFilterQuery(e.target.value)}
                      placeholder="e.g. (title LIKE '%Engineer%' OR title LIKE '%Developer%') AND location LIKE '%Remote%'"
                      className="text-xs font-mono"
                    />
                  </Field>
                  <div className="text-[10px] text-[var(--color-ink)]/70 font-mono mt-0.5">
                    Supports SQL LIKE wildcards (%, _), AND, OR, NOT, IN (...), and field targeting (title, location, company, description).
                  </div>
                </div>
              </div>

              {/* Data Retention (TTL) */}
              <div className="flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)]">
                <div className="flex justify-between items-center border-b border-[var(--color-ink)]/30 pb-1">
                  <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
                    DATA RETENTION (TTL)
                  </span>
                  <Tooltip content="Strict safety guardrail: jobs marked as favorited (★), applications in progress, or with notes are NEVER pruned.">
                    <span className="text-[10px] font-bold text-[var(--color-lime)] cursor-help">
                      [★ SAFETY SHIELD ACTIVE]
                    </span>
                  </Tooltip>
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <Field label="RETENTION [DAYS]">
                    <Input
                      type="number"
                      min="0"
                      max="3650"
                      value={String(retentionDays)}
                      onChange={(e) => setRetentionDays(Math.max(0, parseInt(e.target.value) || 0))}
                      className="text-xs"
                    />
                  </Field>

                  <Field label="POLICY">
                    <Select
                      value={retentionPolicy}
                      onChange={(e) => setRetentionPolicy(e.target.value as "untracked_only" | "all")}
                      options={[
                        { value: "untracked_only", label: "UNTRACKED ONLY (SAFE)" },
                        { value: "all", label: "ALL UNTOUCHED" },
                      ]}
                    />
                  </Field>
                </div>
              </div>

              {/* Custom Selectors for kind === 'selector' */}
              {kind === "selector" && (
                <div className="flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper)]">
                  <span className="font-bold uppercase tracking-wider text-[var(--color-ink)] border-b border-[var(--color-ink)]/30 pb-1">
                    CSS SELECTORS & REGEX
                  </span>
                  <div className="grid grid-cols-2 gap-2">
                    <Field label="ITEM CONTAINER">
                      <Input
                        variant="mono"
                        value={selectorItem}
                        onChange={(e) => setSelectorItem(e.target.value)}
                        placeholder=".job-row"
                        className="text-xs"
                      />
                    </Field>
                    <Field label="TITLE SELECTOR">
                      <Input
                        variant="mono"
                        value={selectorTitle}
                        onChange={(e) => setSelectorTitle(e.target.value)}
                        placeholder=".title"
                        className="text-xs"
                      />
                    </Field>
                    <Field label="LINK SELECTOR">
                      <Input
                        variant="mono"
                        value={selectorLink}
                        onChange={(e) => setSelectorLink(e.target.value)}
                        placeholder="a"
                        className="text-xs"
                      />
                    </Field>
                    <Field label="LOCATION SELECTOR">
                      <Input
                        variant="mono"
                        value={selectorLocation}
                        onChange={(e) => setSelectorLocation(e.target.value)}
                        placeholder=".location"
                        className="text-xs"
                      />
                    </Field>
                  </div>
                </div>
              )}
            </div>

            {/* Right Column (5 cols): Live Dry-Run Pre-Flight Console */}
            <div className="col-span-5 flex flex-col gap-2 p-2.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
              <div className="flex justify-between items-center border-b border-[var(--color-ink)] pb-1.5">
                <span className="font-bold uppercase tracking-wider text-[var(--color-ink)] text-[11px]">
                  DRY-RUN EXTRACTION CONSOLE
                </span>
                <Tooltip content="Spawn sidecar runner and inspect extracted jobs without saving to SQLite">
                  <Button
                    variant="default"
                    onClick={handleRunTransientDryRun}
                    disabled={isDryRunning || (!identifier.trim() && !url.trim())}
                  >
                    {isDryRunning ? "RUNNING…" : "RUN PREVIEW"}
                  </Button>
                </Tooltip>
              </div>

              <div className="flex-1 min-h-[320px] max-h-[460px] overflow-y-auto p-2 border border-[var(--color-ink)] bg-[var(--color-paper)] font-mono text-[11px]">
                {isDryRunning ? (
                  <div className="p-4 text-center text-[var(--color-ink-mute)]">
                    EXTRACTION IN PROGRESS…
                  </div>
                ) : !dryRunEvents ? (
                  <div className="p-4 text-center text-[var(--color-ink-mute)]">
                    CLICK &quot;RUN PREVIEW&quot; TO TEST WORKFLOW AGAINST TARGET URL.
                  </div>
                ) : dryRunEvents.length === 0 ? (
                  <div className="p-4 text-center text-[var(--color-ink-mute)]">
                    NO JOBS MATCHED BY THIS EXTRACTOR CONFIGURATION.
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5">
                    {dryRunEvents.map((ev, i) => (
                      <div key={i} className="p-1 border border-[var(--color-ink)]/20 text-[10px]">
                        {ev.type === "job" ? (
                          <div>
                            <strong className="text-[var(--color-ink)]">{ev.title}</strong>
                            <div className="text-[var(--color-ink-mute)]">
                              {ev.location || "REMOTE"} · {ev.role_type}
                              {ev.salary?.raw ? ` · ${ev.salary.raw}` : ""}
                            </div>
                          </div>
                        ) : ev.type === "error" ? (
                          <div className="text-[var(--color-cherry)] font-bold">
                            ERR: {ev.message}
                          </div>
                        ) : (
                          <div className="text-[var(--color-ink-mute)]">
                            EVENT: {ev.type}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-ink)]/20">
                <Button variant="default" onClick={onClose} disabled={isSaving}>
                  CANCEL
                </Button>
                <Button variant="inverted" onClick={handleSave} disabled={isSaving || !name.trim() || (!identifier.trim() && !url.trim())}>
                  {isSaving ? "SAVING…" : isEditing ? "UPDATE WORKFLOW" : "SAVE WORKFLOW"}
                </Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
