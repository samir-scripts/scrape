import { useUIStore } from "../../lib/store";
import { NAV_TABS, type NavTabId } from "../../lib/constants";

interface TopNavProps {
  discoveredCount?: number;
  trackedCount?: number;
  nextRunText?: string;
}

export function TopNav({
  discoveredCount = 0,
  trackedCount = 0,
  nextRunText,
}: TopNavProps) {
  const { view, setView, isScanning, scanStatusText } = useUIStore();

  return (
    <header className="flex flex-col w-full bg-[var(--color-paper)] shrink-0 select-none">
      <div className="flex items-stretch justify-between h-10 px-3 border-b border-[var(--color-ink)]">
        {/* Left: System Title & Tabs */}
        <div className="flex items-stretch gap-1">
          <div className="flex items-center font-mono font-black text-xs tracking-tight pr-4 border-r border-[var(--color-ink)] my-1.5 text-[var(--color-ink)]">
            SCRAPE<span className="text-[var(--color-ink-mute)] font-normal ml-1">// SYS.02</span>
          </div>

          <nav className="flex items-stretch gap-1 ml-2">
            {NAV_TABS.map((tab) => {
              const isActive = view === tab.id;
              let label: string = tab.label;
              if (tab.id === "discovered") {
                label = `DISCOVERED (${discoveredCount})`;
              } else if (tab.id === "tracked") {
                label = `TRACKED (${trackedCount})`;
              }

              return (
                <button
                  key={tab.id}
                  onClick={() => setView(tab.id as NavTabId)}
                  className={`px-3 my-1.5 flex items-center justify-center font-mono text-xs font-bold tracking-wider transition-none cursor-pointer border ${
                    isActive
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                      : "bg-transparent text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)] hover:border-[var(--color-ink)]"
                  }`}
                >
                  {label}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Right: Status Indicator / Next Run / Scanning */}
        <div className="flex items-center font-mono text-xs text-[var(--color-ink)] pl-4">
          {isScanning ? (
            <div className="flex items-center gap-2 bg-[var(--color-signal)] text-white px-2.5 py-1 text-[11px] font-bold">
              <span className="animate-pulse">●</span>
              <span>{scanStatusText || "SCRAPING IN PROGRESS…"}</span>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-[var(--color-ink-mute)] text-[11px]">
              <span className="text-[var(--color-lime)]">●</span>
              <span>{nextRunText || "IDLE"}</span>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
