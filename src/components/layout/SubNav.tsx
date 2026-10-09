import { useUIStore } from "../../lib/store";
import { ROLE_TYPES, ROLE_TYPE_LABELS } from "../../lib/constants";
import { KeyHint } from "../ui/KeyHint";

export function SubNav() {
  const {
    view,
    roleTypeFilter,
    setRoleTypeFilter,
    trackedViewMode,
    setTrackedViewMode,
    searchQuery,
    setSearchQuery,
  } = useUIStore();

  return (
    <div className="flex items-center justify-between h-9 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 shrink-0 select-none">
      {/* Left: View-specific sub-categories */}
      <div className="flex items-center gap-4 text-xs font-mono">
        {view === "discovered" && (
          <div className="flex items-center gap-1">
            {ROLE_TYPES.map((rt) => {
              const isSelected =
                rt === "all" ? roleTypeFilter === null : roleTypeFilter === rt;

              return (
                <button
                  key={rt}
                  onClick={() => setRoleTypeFilter(rt === "all" ? null : rt)}
                  className={`px-2 py-0.5 uppercase tracking-wider text-xs cursor-pointer border ${
                    isSelected
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold border-[var(--color-ink)]"
                      : "text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)] hover:border-[var(--color-ink-mute)]"
                  }`}
                >
                  {ROLE_TYPE_LABELS[rt] || rt}
                </button>
              );
            })}
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

        {view === "settings" && (
          <span className="text-[11px] font-mono uppercase text-[var(--color-ink-mute)] tracking-wider">
            SYSTEM CONFIGURATION & SCHEDULE
          </span>
        )}

        {view === "metrics" && (
          <span className="text-[11px] font-mono uppercase text-[var(--color-ink-mute)] tracking-wider">
            APPLICATION & CONVERSION METRICS
          </span>
        )}
      </div>

      {/* Right: Quick Search input */}
      {(view === "discovered" || view === "tracked") && (
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
  );
}
