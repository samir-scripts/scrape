import { useUIStore } from "../../lib/store";
import { KeyHint } from "../ui/KeyHint";
import { FilterChips } from "../filters/FilterChips";
import { countActiveFilters } from "../../lib/filter";
import { Tooltip } from "../ui/Tooltip";

export function SubNav() {
  const {
    view,
    trackedViewMode,
    setTrackedViewMode,
    selectedJobId,
    setSelectedJobId,
    filterSpec,
    setFilterSpec,
    resetFilterSpec,
    isFilterPanelOpen,
    toggleFilterPanel,
  } = useUIStore();

  const activeCount = countActiveFilters(filterSpec);
  const showFilters = view === "discovered" || view === "tracked";

  return (
    <div className="flex flex-col w-full shrink-0 select-none">
      <div className="flex items-center justify-between h-9 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40">
        {/* Left: View-specific sub-categories & consolidated Filters button */}
        <div className="flex items-center gap-4 text-xs font-mono">
          {view === "discovered" && (
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-mono uppercase tracking-wider text-[var(--color-ink-mute)]">
                DISCOVERED
              </span>
              {selectedJobId && (
                <button
                  onClick={() => setSelectedJobId(null)}
                  className="flex items-center gap-1 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-paper-2)] text-[10px] font-mono uppercase font-bold text-[var(--color-ink)] cursor-pointer"
                  title="Close active job window [Esc]"
                >
                  <span>✕ CLOSE WINDOW</span>
                  <KeyHint keys="ESC" className="text-[9px]" />
                </button>
              )}
            </div>
          )}

          {view === "tracked" && (
            <div className="flex items-center gap-1">
              <Tooltip content="Switch to pipeline Kanban board view" position="bottom">
                <button
                  onClick={() => setTrackedViewMode("kanban")}
                  className={`px-2.5 py-0.5 uppercase tracking-wider text-xs cursor-pointer border ${
                    trackedViewMode === "kanban"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold border-[var(--color-ink)]"
                      : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
                  }`}
                >
                  [■] KANBAN
                </button>
              </Tooltip>
              <Tooltip content="Switch to structured application table view" position="bottom">
                <button
                  onClick={() => setTrackedViewMode("list")}
                  className={`px-2.5 py-0.5 uppercase tracking-wider text-xs cursor-pointer border ${
                    trackedViewMode === "list"
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold border-[var(--color-ink)]"
                      : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)]"
                  }`}
                >
                  [≡] LIST
                </button>
              </Tooltip>
            </div>
          )}

          {/* Filter Dropdown Toggle Button */}
          {showFilters && (
            <Tooltip content="Toggle query-level criteria dropdown [F]" position="bottom">
              <button
                id="subnav-filters-trigger"
                onClick={toggleFilterPanel}
                className={`flex items-center gap-1.5 px-2.5 py-0.5 border text-xs font-mono font-bold uppercase tracking-wider cursor-pointer ${
                  isFilterPanelOpen || activeCount > 0
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <span>FILTERS {activeCount > 0 ? `(${activeCount})` : ""}</span>
                <KeyHint
                  keys="F"
                  className={`text-[9px] ${
                    isFilterPanelOpen || activeCount > 0
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
                className={`flex items-center gap-1.5 px-2.5 py-0.5 border text-xs font-mono font-bold uppercase tracking-wider cursor-pointer ${
                  filterSpec.favorited_only
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <span>{filterSpec.favorited_only ? "★ FAVORITES" : "☆ FAVORITES"}</span>
              </button>
            </Tooltip>
          )}

          {view === "settings" && (
            <span className="text-[11px] font-mono uppercase text-[var(--color-ink-mute)] tracking-wider">
              SETTINGS
            </span>
          )}

          {view === "metrics" && (
            <span className="text-[11px] font-mono uppercase text-[var(--color-ink-mute)] tracking-wider">
              METRICS
            </span>
          )}
        </div>

        {/* Right: Quick Search input */}
        {showFilters && (
          <div className="flex items-center gap-2">
            <div className="relative flex items-center">
              <input
                id="global-search-input"
                type="text"
                placeholder="SEARCH (TITLE, CO, LOC)..."
                value={filterSpec.search || ""}
                onChange={(e) =>
                  setFilterSpec((prev) => ({
                    ...prev,
                    search: e.target.value.trim() ? e.target.value : null,
                  }))
                }
                className="w-48 px-2 py-0.5 text-xs font-mono uppercase bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)]"
              />
              {filterSpec.search ? (
                <button
                  onClick={() =>
                    setFilterSpec((prev) => ({ ...prev, search: null }))
                  }
                  className="absolute right-1 text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] px-1 cursor-pointer"
                >
                  ×
                </button>
              ) : (
                <KeyHint
                  keys="/"
                  className="absolute right-1.5 opacity-60 text-[9px] pointer-events-none"
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
          onClearAll={resetFilterSpec}
        />
      )}
    </div>
  );
}
