import { useEffect, useState } from "react";
import type { JobFilter } from "../../bindings/JobFilter";
import type { FilterImpactPreview } from "../../bindings/FilterImpactPreview";
import { api } from "../../lib/api";
import { SalaryRangeFilter } from "./SalaryRangeFilter";
import { ExperienceFilter } from "./ExperienceFilter";
import { NewGradFilter } from "./NewGradFilter";
import { LocationFilter } from "./LocationFilter";
import { TriStateFilter } from "./TriStateFilter";
import { RoleTypeFilter } from "./RoleTypeFilter";
import { Rule } from "../ui/Rule";

export interface FilterPanelProps {
  isOpen: boolean;
  onClose: () => void;
  filterSpec: Partial<JobFilter>;
  onUpdateFilter: (updater: (prev: Partial<JobFilter>) => Partial<JobFilter>) => void;
  onResetFilters: () => void;
}

export function FilterPanel({
  isOpen,
  onClose,
  filterSpec,
  onUpdateFilter,
  onResetFilters,
}: FilterPanelProps) {
  const [impact, setImpact] = useState<FilterImpactPreview | null>(null);

  // Keyboard shortcut listener for Esc
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.shiftKey && (e.key === "F" || e.key === "f")) {
        e.preventDefault();
        onResetFilters();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose, onResetFilters]);

  // Preview impact query on filter change
  useEffect(() => {
    if (!isOpen) return;
    let isCancelled = false;

    // Send complete JobFilter object to preview_filter_impact
    const fullSpec: JobFilter = {
      statuses: filterSpec.statuses ?? null,
      role_types: filterSpec.role_types ?? null,
      search: filterSpec.search ?? null,
      source_id: filterSpec.source_id ?? null,
      sort: filterSpec.sort ?? null,
      limit: null,
      offset: null,
      salary_min: filterSpec.salary_min ?? null,
      salary_max: filterSpec.salary_max ?? null,
      salary_currency: filterSpec.salary_currency ?? null,
      salary_include_unknown: filterSpec.salary_include_unknown ?? true,
      experience_min: filterSpec.experience_min ?? null,
      experience_max: filterSpec.experience_max ?? null,
      experience_include_unknown: filterSpec.experience_include_unknown ?? true,
      new_grad: filterSpec.new_grad ?? null,
      sponsorship: filterSpec.sponsorship ?? null,
      clearance: filterSpec.clearance ?? null,
      countries: filterSpec.countries ?? null,
      regions: filterSpec.regions ?? null,
      remote: filterSpec.remote ?? null,
      location_include_unknown: filterSpec.location_include_unknown ?? true,
    };

    api.previewFilterImpact(fullSpec)
      .then((res) => {
        if (!isCancelled) setImpact(res);
      })
      .catch(() => {
        // Fallback
      });

    return () => {
      isCancelled = true;
    };
  }, [isOpen, filterSpec]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/40 backdrop-blur-[1px] select-none">
      <div className="w-full max-w-md h-full bg-[var(--color-paper)] border-l-2 border-[var(--color-ink)] flex flex-col shadow-none animate-none">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40">
          <div>
            <h2 className="font-mono text-sm font-black uppercase tracking-wider text-[var(--color-ink)]">
              FILTER ENGINE
            </h2>
            <p className="font-mono text-[10px] text-[var(--color-ink-mute)] uppercase">
              QUERY-LEVEL CRITERIA · HOTKEY [ESC]
            </p>
          </div>
          <button
            onClick={onClose}
            className="w-7 h-7 flex items-center justify-center border border-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] font-mono font-bold text-sm cursor-pointer"
            aria-label="Close filters"
          >
            ×
          </button>
        </div>

        {/* Scrollable Filter Form */}
        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-5">
          {/* Employment Type / Role Filter */}
          <RoleTypeFilter
            selected={filterSpec.role_types ?? null}
            onChange={(roles) =>
              onUpdateFilter((prev) => ({ ...prev, role_types: roles }))
            }
          />

          <Rule />

          {/* Salary Filter */}
          <SalaryRangeFilter
            salaryMin={filterSpec.salary_min ?? null}
            salaryMax={filterSpec.salary_max ?? null}
            currency={filterSpec.salary_currency ?? null}
            includeUnknown={filterSpec.salary_include_unknown ?? true}
            onChange={({ salaryMin, salaryMax, currency, includeUnknown }) =>
              onUpdateFilter((prev) => ({
                ...prev,
                salary_min: salaryMin,
                salary_max: salaryMax,
                salary_currency: currency,
                salary_include_unknown: includeUnknown,
              }))
            }
          />

          <Rule />

          {/* Experience Filter */}
          <ExperienceFilter
            experienceMin={filterSpec.experience_min ?? null}
            experienceMax={filterSpec.experience_max ?? null}
            includeUnknown={filterSpec.experience_include_unknown ?? true}
            onChange={({ experienceMin, experienceMax, includeUnknown }) =>
              onUpdateFilter((prev) => ({
                ...prev,
                experience_min: experienceMin,
                experience_max: experienceMax,
                experience_include_unknown: includeUnknown,
              }))
            }
          />

          <Rule />

          {/* New Grad Filter */}
          <NewGradFilter
            value={filterSpec.new_grad ?? "any"}
            onChange={(val) =>
              onUpdateFilter((prev) => ({ ...prev, new_grad: val }))
            }
          />

          <Rule />

          {/* Location Filter */}
          <LocationFilter
            countries={filterSpec.countries ?? null}
            regions={filterSpec.regions ?? null}
            remote={filterSpec.remote ?? null}
            includeUnknown={filterSpec.location_include_unknown ?? true}
            onChange={({ countries, regions, remote, includeUnknown }) =>
              onUpdateFilter((prev) => ({
                ...prev,
                countries,
                regions,
                remote,
                location_include_unknown: includeUnknown,
              }))
            }
          />

          <Rule />

          {/* Sponsorship Filter */}
          <TriStateFilter
            label="VISA SPONSORSHIP"
            description="FILTER BY EMPLOYER SPONSORSHIP POLICIES"
            selected={filterSpec.sponsorship ?? null}
            onChange={(vals) =>
              onUpdateFilter((prev) => ({ ...prev, sponsorship: vals }))
            }
          />

          <Rule />

          {/* Security Clearance Filter */}
          <TriStateFilter
            label="SECURITY CLEARANCE"
            description="FILTER BY GOVERNMENT CLEARANCE REQUIREMENTS"
            selected={filterSpec.clearance ?? null}
            onChange={(vals) =>
              onUpdateFilter((prev) => ({ ...prev, clearance: vals }))
            }
          />
        </div>

        {/* Footer with Impact Preview and Buttons */}
        <div className="p-4 border-t border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 flex flex-col gap-3 font-mono">
          {impact && (
            <div className="flex justify-between items-center text-xs">
              <span className="text-[var(--color-ink-mute)] uppercase">
                FILTER IMPACT:
              </span>
              <span className="font-bold text-[var(--color-ink)]">
                {impact.kept} OF {impact.total} POSTINGS MATCH
              </span>
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={onResetFilters}
              className="flex-1 py-2 px-3 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-xs font-bold uppercase cursor-pointer"
            >
              [RESET ALL]
            </button>
            <button
              onClick={onClose}
              className="flex-1 py-2 px-3 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-90 text-xs font-bold uppercase cursor-pointer"
            >
              [APPLY & CLOSE]
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
