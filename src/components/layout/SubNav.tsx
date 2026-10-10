import { useUIStore } from "../../lib/store";
import { ROLE_TYPES, ROLE_TYPE_LABELS } from "../../lib/constants";
import { KeyHint } from "../ui/KeyHint";
import { Select } from "../ui/Select";
import { FilterChips } from "../filters/FilterChips";
import { countActiveFilters } from "../../lib/filter";

export function SubNav() {
  const {
    view,
    roleTypeFilter,
    setRoleTypeFilter,
    trackedViewMode,
    setTrackedViewMode,
    searchQuery,
    setSearchQuery,
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
        {/* Left: View-specific sub-categories */}
        <div className="flex items-center gap-4 text-xs font-mono">
          {view === "discovered" && (
            <div className="flex items-center gap-2">
              <span className="text-[10px] text-[var(--color-ink-mute)] font-bold uppercase tracking-wider">
                ROLE:
              </span>
              <Select
                value={roleTypeFilter || "all"}
                onChange={(e) =>
                  setRoleTypeFilter(e.target.value === "all" ? null : e.target.value)
                }
                options={ROLE_TYPES.map((rt) => ({
                  value: rt,
                  label: ROLE_TYPE_LABELS[rt] || rt.toUpperCase(),
                }))}
                className="py-0.5 text-xs bg-[var(--color-paper)] h-7"
              />
            </div>
          )}

          {view === "tracked" && (
            <div className="flex items-center gap-1">
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
            </div>
          )}

          {/* Filter Drawer Toggle Button */}
          {showFilters && (
            <button
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
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-48 px-2 py-0.5 text-xs font-mono uppercase bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)]"
              />
              {searchQuery ? (
                <button
                  onClick={() => setSearchQuery("")}
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
