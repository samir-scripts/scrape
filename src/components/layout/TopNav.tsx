import { useUIStore } from "../../lib/store";
import { NAV_TABS, type NavTabId } from "../../lib/constants";
import { Tooltip } from "../ui/Tooltip";
import { NotificationBell } from "../notifications/NotificationBell";

interface TopNavProps {
  discoveredCount?: number;
  trackedCount?: number;
  nextRunText?: string;
}

const TAB_TOOLTIPS: Record<NavTabId, string> = {
  discovered: "Unprocessed postings intake queue",
  tracked: "Application pipeline & Kanban board",
  metrics: "Analytics, conversions, and salary benchmarks",
  scraper: "Autonomous workflows, runners, and telemetry",
  settings: "System preferences, appearance, and global options",
};

export function TopNav({
  discoveredCount = 0,
  trackedCount = 0,
  nextRunText,
}: TopNavProps) {
  const { view, setView, isScanning, scanStatusText } = useUIStore();

  return (
    <header className="flex flex-col w-full bg-[var(--color-paper)] shrink-0 select-none">
      <div className="flex items-stretch justify-between h-10 px-3 border-b border-[var(--color-ink)]">
        {/* Left: Tabs */}
        <div className="flex items-stretch gap-1">
          <nav className="flex items-stretch gap-1">
            {NAV_TABS.map((tab) => {
              const isActive = view === tab.id;
              let label: string = tab.label;
              if (tab.id === "discovered") {
                label = `DISCOVERED (${discoveredCount})`;
              } else if (tab.id === "tracked") {
                label = `TRACKED (${trackedCount})`;
              }

              return (
                <Tooltip
                  key={tab.id}
                  content={TAB_TOOLTIPS[tab.id as NavTabId]}
                  position="bottom"
                  className="flex items-stretch"
                >
                  <button
                    onClick={() => setView(tab.id as NavTabId)}
                    className={`px-3 my-1.5 flex items-center justify-center font-mono text-xs font-bold tracking-wider transition-none cursor-pointer border ${
                      isActive
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                        : "bg-transparent text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)] hover:border-[var(--color-ink)]"
                    }`}
                  >
                    {label}
                  </button>
                </Tooltip>
              );
            })}
          </nav>
        </div>

        {/* Right: Status Indicator / Next Run / Scanning & Notification Bell */}
        <div className="flex items-center gap-3 font-mono text-xs text-[var(--color-ink)] pl-4">
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

          <div className="h-4 w-px bg-[var(--color-ink-mute)]/30" />

          <NotificationBell />
        </div>
      </div>
    </header>
  );
}
