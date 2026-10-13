import { useEffect, useState, useRef } from "react";
import { createPortal } from "react-dom";
import type { JobFilter } from "../../bindings/JobFilter";
import type { FilterImpactPreview } from "../../bindings/FilterImpactPreview";
import { api } from "../../lib/api";
import { countActiveFilters } from "../../lib/filter";
import { SalaryRangeFilter } from "./SalaryRangeFilter";
import { ExperienceFilter } from "./ExperienceFilter";
import { LocationFilter } from "./LocationFilter";
import { TriStateFilter } from "./TriStateFilter";
import { RoleTypeFilter } from "./RoleTypeFilter";
import { Rule } from "../ui/Rule";
export {
  calculateFilterDropdownCoordinates,
  type FilterDropdownCoordinates,
} from "../../lib/positioning";
import {
  calculateFilterDropdownCoordinates,
  type FilterDropdownCoordinates,
} from "../../lib/positioning";

export interface FilterPanelProps {
  isOpen: boolean;
  onClose: () => void;
  filterSpec: Partial<JobFilter>;
  onUpdateFilter: (updater: (prev: Partial<JobFilter>) => Partial<JobFilter>) => void;
  onResetFilters: () => void;
  anchorId?: string;
  anchorEl?: HTMLElement | null;
}

export function FilterPanel({
  isOpen,
  onClose,
  filterSpec,
  onUpdateFilter,
  onResetFilters,
  anchorId = "subnav-filters-trigger",
  anchorEl,
}: FilterPanelProps) {
  const [impact, setImpact] = useState<FilterImpactPreview | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const [coords, setCoords] = useState<FilterDropdownCoordinates>(() =>
    calculateFilterDropdownCoordinates(null)
  );

  const activeCount = countActiveFilters(filterSpec);

  // Position calculation engine
  const updatePosition = () => {
    if (typeof window === "undefined") return;
    const trigger =
      anchorEl ||
      document.getElementById(anchorId);
    if (trigger) {
      const rect = trigger.getBoundingClientRect();
      setCoords(
        calculateFilterDropdownCoordinates(
          rect,
          window.innerWidth,
          window.innerHeight
        )
      );
    } else {
      setCoords(
        calculateFilterDropdownCoordinates(
          null,
          window.innerWidth,
          window.innerHeight
        )
      );
    }
  };

  // Re-measure on open, window resize, and scroll
  useEffect(() => {
    if (!isOpen) return;
    updatePosition();

    const handleResize = () => updatePosition();
    const handleScroll = () => updatePosition();

    window.addEventListener("resize", handleResize);
    window.addEventListener("scroll", handleScroll, true);

    return () => {
      window.removeEventListener("resize", handleResize);
      window.removeEventListener("scroll", handleScroll, true);
    };
  }, [isOpen, anchorEl, anchorId]);

  // Light-dismiss click outside detector
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent | PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;

      // Clicked inside the dropdown panel
      if (panelRef.current && panelRef.current.contains(target)) {
        return;
      }

      // Clicked on the trigger button itself (let button's onClick toggle it closed without double-toggle race)
      const trigger =
        anchorEl ||
        document.getElementById(anchorId);
      if (trigger && trigger.contains(target)) {
        return;
      }

      onClose();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [isOpen, onClose, anchorEl, anchorId]);

  // Keyboard shortcut listener for Esc and Shift+F (capture phase for priority)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      } else if (e.shiftKey && (e.key === "F" || e.key === "f")) {
        e.preventDefault();
        e.stopPropagation();
        onResetFilters();
      }
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
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
      favorited_only: filterSpec.favorited_only ?? null,
      salary_min: filterSpec.salary_min ?? null,
      salary_max: filterSpec.salary_max ?? null,
      salary_currency: filterSpec.salary_currency ?? null,
      salary_include_unknown: filterSpec.salary_include_unknown ?? false,
      salary_interval: filterSpec.salary_interval ?? null,
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
      work_auth: filterSpec.work_auth ?? null,
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

  const content = (
    <div
      ref={panelRef}
      id="subnav-filters-dropdown"
      role="dialog"
      aria-modal="false"
      aria-label="Filter Engine"
      style={{
        position: "fixed",
        top: `${coords.top}px`,
        left: `${coords.left}px`,
        width: `${coords.width}px`,
        maxHeight: `${coords.maxHeight}px`,
      }}
      className="z-50 bg-[var(--color-paper)] border-2 border-[var(--color-ink)] shadow-none flex flex-col select-none animate-none"
    >
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/50 shrink-0">
        <div className="flex items-center gap-2 font-mono">
          <span className="text-xs font-black uppercase tracking-wider text-[var(--color-ink)]">
            FILTER ENGINE
          </span>
          {activeCount > 0 && (
            <span className="bg-[var(--color-ink)] text-[var(--color-paper)] px-1.5 py-0.5 text-[9px] font-bold">
              {activeCount} ACTIVE
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5 font-mono text-xs">
          {activeCount > 0 && (
            <button
              type="button"
              onClick={onResetFilters}
              className="px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[var(--color-ink)] uppercase font-bold cursor-pointer"
            >
              [RESET]
            </button>
          )}
          <button
            type="button"
            onClick={onClose}
            className="flex items-center gap-1 px-1.5 py-0.5 border border-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] font-bold cursor-pointer"
            aria-label="Close filters"
          >
            <span>[ESC]</span>
            <span className="text-xs leading-none font-bold">×</span>
          </button>
        </div>
      </div>

      {/* Scrollable Filter Form */}
      <div className="flex-1 overflow-y-auto p-3.5 flex flex-col gap-4">
        {/* Contains / Keywords Filter */}
        <div className="flex flex-col gap-1.5 font-mono text-xs select-none">
          <div className="flex justify-between items-center">
            <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
              CONTAINS (KEYWORDS / TEXT)
            </label>
            {Boolean(filterSpec.search?.trim()) && (
              <button
                type="button"
                onClick={() =>
                  onUpdateFilter((prev) => ({ ...prev, search: null }))
                }
                className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
              >
                [RESET]
              </button>
            )}
          </div>
          <div className="flex items-center border border-[var(--color-ink)] bg-[var(--color-paper)]">
            <input
              type="text"
              placeholder="FILTER BY TITLE, COMPANY, SKILLS, TEXT..."
              value={filterSpec.search ?? ""}
              onChange={(e) => {
                const val = e.target.value;
                onUpdateFilter((prev) => ({
                  ...prev,
                  search: val.trim().length > 0 ? val : null,
                }));
              }}
              className="w-full py-1 px-2 bg-transparent text-[var(--color-ink)] text-xs font-mono uppercase focus:outline-none placeholder-[var(--color-ink-mute)]"
            />
            {Boolean(filterSpec.search?.trim()) && (
              <button
                type="button"
                onClick={() =>
                  onUpdateFilter((prev) => ({ ...prev, search: null }))
                }
                className="px-2 text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] cursor-pointer"
              >
                ×
              </button>
            )}
          </div>
        </div>

        <Rule />

        {/* Employment Type / Role Filter */}
        <RoleTypeFilter
          selected={filterSpec.role_types ?? null}
          onChange={(roles) =>
            onUpdateFilter((prev) => ({ ...prev, role_types: roles }))
          }
        />

        <Rule />

        {/* Salary / Compensation Filter */}
        <SalaryRangeFilter
          salaryMin={filterSpec.salary_min ?? null}
          salaryMax={filterSpec.salary_max ?? null}
          currency={filterSpec.salary_currency ?? null}
          interval={filterSpec.salary_interval ?? null}
          includeUnknown={filterSpec.salary_include_unknown ?? false}
          onChange={({ salaryMin, salaryMax, currency, interval, includeUnknown }) =>
            onUpdateFilter((prev) => ({
              ...prev,
              salary_min: salaryMin,
              salary_max: salaryMax,
              salary_currency: currency,
              salary_interval: interval,
              salary_include_unknown: includeUnknown,
            }))
          }
        />

        <Rule />

        {/* Experience & Career Level Filter (Consolidated) */}
        <ExperienceFilter
          experienceMin={filterSpec.experience_min ?? null}
          experienceMax={filterSpec.experience_max ?? null}
          newGrad={filterSpec.new_grad ?? "any"}
          includeUnknown={filterSpec.experience_include_unknown ?? true}
          onChange={({ experienceMin, experienceMax, newGrad, includeUnknown }) =>
            onUpdateFilter((prev) => ({
              ...prev,
              experience_min: experienceMin,
              experience_max: experienceMax,
              new_grad: newGrad,
              experience_include_unknown: includeUnknown,
            }))
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
      <div className="p-3 border-t border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 flex flex-col gap-2 font-mono shrink-0">
        {impact && (
          <div className="flex justify-between items-center text-xs">
            <span className="text-[var(--color-ink-mute)] uppercase">
              FILTER IMPACT:
            </span>
            <span className="font-bold text-[var(--color-ink)]">
              {impact.kept} OF {impact.total} MATCH
            </span>
          </div>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            onClick={onResetFilters}
            className="flex-1 py-1.5 px-2 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-xs font-bold uppercase cursor-pointer"
          >
            [RESET ALL]
          </button>
          <button
            type="button"
            onClick={onClose}
            className="flex-1 py-1.5 px-2 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] hover:opacity-90 text-xs font-bold uppercase cursor-pointer"
          >
            [APPLY & CLOSE]
          </button>
        </div>
      </div>
    </div>
  );

  if (typeof document !== "undefined" && document.body) {
    return createPortal(content, document.body);
  }

  return content;
}
