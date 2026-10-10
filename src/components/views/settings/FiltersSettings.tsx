import { useState } from "react";
import type { Settings } from "../../../bindings/Settings";
import type { RoleType } from "../../../bindings/RoleType";
import { Input } from "../../ui/Input";
import { Rule } from "../../ui/Rule";
import { Toggle } from "../../ui/Toggle";
import { ROLE_TYPES, ROLE_TYPE_LABELS } from "../../../lib/constants";
import { api } from "../../../lib/api";

interface FiltersSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function FiltersSettings({
  settings,
  onUpdateSetting,
}: FiltersSettingsProps) {
  const [includeInput, setIncludeInput] = useState("");
  const [excludeInput, setExcludeInput] = useState("");
  const [locationInput, setLocationInput] = useState("");

  const handleAddChip = (
    key: string,
    currentList: string[],
    newVal: string,
    setter: (v: string) => void
  ) => {
    const trimmed = newVal.trim();
    if (!trimmed) return;
    if (!currentList.includes(trimmed)) {
      onUpdateSetting(key, [...currentList, trimmed]);
    }
    setter("");
  };

  const handleRemoveChip = (key: string, currentList: string[], toRemove: string) => {
    onUpdateSetting(
      key,
      currentList.filter((item) => item !== toRemove)
    );
  };

  const handleToggleRoleType = (rt: RoleType, checked: boolean) => {
    const current = settings.filters_role_types || [];
    if (checked) {
      if (!current.includes(rt)) {
        onUpdateSetting("filters.role_types", [...current, rt]);
      }
    } else {
      onUpdateSetting(
        "filters.role_types",
        current.filter((item) => item !== rt)
      );
    }
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl select-none">
      <div>
        <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
          GLOBAL KEYWORD & ROLE FILTERS
        </h2>
        <p className="text-xs font-mono text-[var(--color-ink-mute)]">
          POSTINGS ARE FILTERED ON INGESTION BY THE SCRAPE WORKER SIDECAR
        </p>
      </div>

      <Rule />

      {/* Role Types Filter */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          TARGET EMPLOYMENT & ROLE TYPES
        </label>
        <div className="grid grid-cols-2 gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
          {ROLE_TYPES.filter((r) => r !== "all").map((rt) => {
            const isChecked = settings.filters_role_types.includes(rt as RoleType);
            return (
              <Toggle
                key={rt}
                checked={isChecked}
                onChange={(checked) =>
                  handleToggleRoleType(rt as RoleType, checked)
                }
                label={ROLE_TYPE_LABELS[rt] || rt}
              />
            );
          })}
        </div>
      </div>

      <Rule />

      {/* Include Keywords */}
      <div className="flex flex-col gap-2 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          INCLUDE KEYWORDS (TITLE MUST CONTAIN AT LEAST ONE, IF SPECIFIED)
        </label>
        <div className="flex gap-2">
          <Input
            value={includeInput}
            onChange={(e) => setIncludeInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleAddChip(
                  "filters.include_keywords",
                  settings.filters_include_keywords,
                  includeInput,
                  setIncludeInput
                );
              }
            }}
            placeholder="TYPE KEYWORD (E.G. RUST, REACT, DATA) AND PRESS ENTER"
            className="flex-1"
          />
        </div>
        <div className="flex flex-wrap gap-1.5 mt-1 min-h-6">
          {settings.filters_include_keywords.length === 0 ? (
            <span className="text-[11px] text-[var(--color-ink-mute)]">
              NO INCLUDE FILTERS (ALL POSTINGS MATCHED)
            </span>
          ) : (
            settings.filters_include_keywords.map((kw) => (
              <span
                key={kw}
                className="inline-flex items-center gap-1.5 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] text-xs font-bold uppercase tracking-wider"
              >
                <span>{kw}</span>
                <button
                  onClick={() =>
                    handleRemoveChip(
                      "filters.include_keywords",
                      settings.filters_include_keywords,
                      kw
                    )
                  }
                  className="hover:text-[var(--color-cherry)] cursor-pointer text-sm leading-none"
                >
                  ×
                </button>
              </span>
            ))
          )}
        </div>
      </div>

      <Rule />

      {/* Exclude Keywords */}
      <div className="flex flex-col gap-2 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          EXCLUDE KEYWORDS (TITLE MATCHING ANY WILL BE SKIPPED)
        </label>
        <div className="flex gap-2">
          <Input
            value={excludeInput}
            onChange={(e) => setExcludeInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleAddChip(
                  "filters.exclude_keywords",
                  settings.filters_exclude_keywords,
                  excludeInput,
                  setExcludeInput
                );
              }
            }}
            placeholder="TYPE KEYWORD (E.G. SENIOR, STAFF, PRINCIPAL, CLEARANCE) AND PRESS ENTER"
            className="flex-1"
          />
        </div>
        <div className="flex flex-wrap gap-1.5 mt-1 min-h-6">
          {settings.filters_exclude_keywords.length === 0 ? (
            <span className="text-[11px] text-[var(--color-ink-mute)]">
              NO EXCLUDE FILTERS CONFIGURED
            </span>
          ) : (
            settings.filters_exclude_keywords.map((kw) => (
              <span
                key={kw}
                className="inline-flex items-center gap-1.5 px-2 py-0.5 border border-[var(--color-cherry)] bg-[var(--color-paper-2)] text-[var(--color-cherry)] text-xs font-bold uppercase tracking-wider"
              >
                <span>{kw}</span>
                <button
                  onClick={() =>
                    handleRemoveChip(
                      "filters.exclude_keywords",
                      settings.filters_exclude_keywords,
                      kw
                    )
                  }
                  className="hover:text-black cursor-pointer text-sm leading-none"
                >
                  ×
                </button>
              </span>
            ))
          )}
        </div>
      </div>

      <Rule />

      {/* Location Filter */}
      <div className="flex flex-col gap-2 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          TARGET LOCATIONS (LOCATION MUST CONTAIN AT LEAST ONE, IF SPECIFIED)
        </label>
        <div className="flex gap-2">
          <Input
            value={locationInput}
            onChange={(e) => setLocationInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleAddChip(
                  "filters.locations",
                  settings.filters_locations,
                  locationInput,
                  setLocationInput
                );
              }
            }}
            placeholder="TYPE LOCATION (E.G. REMOTE, NEW YORK, UNITED STATES) AND PRESS ENTER"
            className="flex-1"
          />
        </div>
        <div className="flex flex-wrap gap-1.5 mt-1 min-h-6">
          {settings.filters_locations.length === 0 ? (
            <span className="text-[11px] text-[var(--color-ink-mute)]">
              NO LOCATION FILTERS (ANY LOCATION ACCEPTED)
            </span>
          ) : (
            settings.filters_locations.map((loc) => (
              <span
                key={loc}
                className="inline-flex items-center gap-1.5 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] text-xs font-bold uppercase tracking-wider"
              >
                <span>{loc}</span>
                <button
                  onClick={() =>
                    handleRemoveChip(
                      "filters.locations",
                      settings.filters_locations,
                      loc
                    )
                  }
                  className="hover:text-[var(--color-cherry)] cursor-pointer text-sm leading-none"
                >
                  ×
                </button>
              </span>
            ))
          )}
        </div>
      </div>

      <Rule />

      {/* Advanced Ingest Controls & Reclassification */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          ADVANCED INGESTION & DATA RECLASSIFICATION
        </label>
        <p className="text-[11px] text-[var(--color-ink-mute)]">
          RE-EXTRACT ENRICHED SALARY, VISA SPONSORSHIP, SECURITY CLEARANCE, AND YOE ON ALL HISTORIC DATABASE POSTINGS.
        </p>

        <div className="p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 flex items-center justify-between">
          <div>
            <div className="font-bold text-xs uppercase text-[var(--color-ink)]">
              RECLASSIFY HISTORIC POSTINGS
            </div>
            <div className="text-[10px] text-[var(--color-ink-mute)] uppercase">
              RUNS NATURAL LANGUAGE ENRICHMENT ENGINE ON EXISTING DESCRIPTIONS
            </div>
          </div>
          <button
            type="button"
            onClick={async () => {
              try {
                const count = await api.reclassifyJobs();
                alert(`Reclassified ${count} job postings.`);
              } catch (e) {
                console.error("Reclassify failed:", e);
              }
            }}
            className="px-3 py-1.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-90 font-mono text-xs font-bold uppercase cursor-pointer"
          >
            [RECLASSIFY JOBS]
          </button>
        </div>
      </div>
    </div>
  );
}
