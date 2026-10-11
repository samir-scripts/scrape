import { useUIStore } from "../../lib/store";
import { NAV_TABS, type NavTabId } from "../../lib/constants";
import { Tooltip } from "../ui/Tooltip";
import { NotificationBell } from "../notifications/NotificationBell";
import { api } from "../../lib/api";

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

          {/* External Terminal Console Launcher */}
          <Tooltip content="Open Live System Console in Terminal.app">
            <button
              onClick={() => api.openTerminalConsole()}
              className="p-1 border border-transparent hover:border-[var(--color-ink)] hover:bg-[var(--color-paper-2)] text-[var(--color-ink)] cursor-pointer flex items-center justify-center transition-none"
              aria-label="Open Terminal Console"
              id="terminal-console-btn"
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
                <rect x="1.5" y="2.5" width="13" height="11" />
                <path d="M4.5 6l2.5 2-2.5 2" />
                <line x1="8.5" y1="10" x2="11.5" y2="10" />
              </svg>
            </button>
          </Tooltip>

          <NotificationBell />
        </div>
      </div>
    </header>
  );
}
