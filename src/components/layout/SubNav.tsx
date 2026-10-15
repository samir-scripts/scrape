import type { JobSort } from "../../bindings/JobSort";
import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
import { useUIStore } from "../../lib/store";
import { KeyHint } from "../ui/KeyHint";
import { FilterChips } from "../filters/FilterChips";
import { countActiveFilters } from "../../lib/filter";
import { Tooltip } from "../ui/Tooltip";
import { contextualJobSort } from "../../lib/jobSort";

export function SubNav({ sort, onResetSort }: { sort?: JobSort; onResetSort?: () => void }) {
  const {
    view,
    trackedViewMode,
    setTrackedViewMode,
    filterSpec,
    setFilterSpec,
    clearFilters,
    isFilterPanelOpen,
    toggleFilterPanel,
  } = useUIStore();

  const activeCount = countActiveFilters(filterSpec);
  const defaultSort = view === "tracked" || view === "discovered" ? contextualJobSort(view) : undefined;
  const customSort = Boolean(sort && defaultSort && sort !== defaultSort);
  const showFilters = view === "discovered" || view === "tracked";

  if (!showFilters) return null;

  return (
    <div className="flex flex-col w-full shrink-0 select-none">
      <div role="toolbar" aria-label="Job controls" className="flex flex-wrap gap-2 items-center justify-between min-h-9 py-1 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40">
        {/* Left: View-specific sub-categories & consolidated Filters button */}
        <div className="flex flex-wrap items-center gap-2 sm:gap-4 text-xs font-mono">
          {view === "tracked" && (
            <div className="flex items-center gap-1">
              <Tooltip content="Switch to pipeline Kanban board view" position="bottom">
                <button
                  onClick={() => setTrackedViewMode("kanban")}
                  className={`inline-flex items-center gap-2 min-h-[var(--control-height-sm)] px-2.5 py-0.5 uppercase tracking-wider text-xs cursor-pointer border ${
                    trackedViewMode === "kanban"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold border-[var(--color-ink)]"
                      : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
                  }`}
                >
                  <Icon name="board" /> KANBAN
                </button>
              </Tooltip>
              <Tooltip content="Switch to structured application table view" position="bottom">
                <button
                  onClick={() => setTrackedViewMode("list")}
                  className={`inline-flex items-center gap-2 min-h-[var(--control-height-sm)] px-2.5 py-0.5 uppercase tracking-wider text-xs cursor-pointer border ${
                    trackedViewMode === "list"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold border-[var(--color-ink)]"
                      : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
                  }`}
                >
                  <Icon name="list" /> LIST
                </button>
              </Tooltip>
            </div>
          )}

          {/* Filter Dropdown Toggle Button */}
          {showFilters && (
              <Tooltip content="Toggle filters and sorting [F]" position="bottom">
              <button
                id="subnav-filters-trigger"
                onClick={toggleFilterPanel}
                className={`flex items-center gap-1.5 min-h-[var(--control-height-sm)] px-2.5 py-0.5 border text-xs font-mono font-bold uppercase tracking-wider cursor-pointer ${
                  isFilterPanelOpen || activeCount > 0 || customSort
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <span>FILTERS &amp; SORT {activeCount > 0 ? `(${activeCount})` : ""}</span>
                <KeyHint
                  keys="F"
                  className={`text-[9px] ${
                    isFilterPanelOpen || activeCount > 0 || customSort
                      ? "border-[var(--color-paper)] text-[var(--color-paper)]"
                      : ""
                  }`}
                />
              </button>
            </Tooltip>
          )}

          {/* Quick Favorites Toggle */}
          {showFilters && (
            <Tooltip content="Filter by favorited postings" position="bottom">
              <button
                onClick={() =>
                  setFilterSpec((prev) => ({
                    ...prev,
                    favorited_only: prev.favorited_only ? null : true,
                  }))
                }
                className={`flex items-center gap-1.5 min-h-[var(--control-height-sm)] px-2.5 py-0.5 border text-xs font-mono font-bold uppercase tracking-wider cursor-pointer ${
                  filterSpec.favorited_only
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <Icon name="star" filled={Boolean(filterSpec.favorited_only)} /><span>FAVORITES</span>
              </button>
            </Tooltip>
          )}

        </div>

        {/* Right: Quick Search input */}
        {showFilters && (
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative flex items-center">
              <input
                id="global-search-input"
                type="text"
                aria-label="Search jobs"
                placeholder="SEARCH (TITLE, CO, LOC)..."
                value={filterSpec.search || ""}
                onChange={(e) =>
                  setFilterSpec((prev) => ({
                    ...prev,
                    search: e.target.value.trim() ? e.target.value : null,
                  }))
                }
                className="w-48 min-h-[var(--control-height-sm)] pl-2 pr-9 py-0.5 text-xs font-mono normal-case bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)]"
              />
              {filterSpec.search ? (
                <IconButton label="Clear search" onClick={() => setFilterSpec((prev) => ({ ...prev, search: null }))}
                  className="absolute right-0"><Icon name="close" size="sm" /></IconButton>
              ) : (
                <KeyHint
                  keys="/"
                  className="absolute right-1.5 opacity-80 text-[9px] pointer-events-none"
                />
              )}
            </div>
          </div>
        )}
      </div>

      {/* Active Filter Chips */}
      {showFilters && (
        <FilterChips
          filter={filterSpec}
          onUpdate={setFilterSpec}
          onClearAll={clearFilters}
          sort={sort}
          defaultSort={defaultSort}
          onResetSort={onResetSort}
        />
      )}
    </div>
  );
}
