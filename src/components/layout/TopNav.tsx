import type { ReactNode } from "react";
import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
import { useUIStore } from "../../lib/store";
import { NAV_TABS, type NavTabId } from "../../lib/constants";
import { Tooltip } from "../ui/Tooltip";
import { NotificationBell } from "../notifications/NotificationBell";
import { api } from "../../lib/api";

interface TopNavProps {
  discoveredCount?: number;
  trackedCount?: number;
  scheduleControl?: ReactNode;
}

const TAB_TOOLTIPS: Record<NavTabId, string> = {
  discovered: "Unprocessed postings intake queue",
  tracked: "Application pipeline & Kanban board",
  metrics: "Analytics, conversions, and salary benchmarks",
  scraper: "Configure workflows",
  settings: "System preferences, appearance, and global options",
};

const TAB_ACTIONS: Record<NavTabId, string> = {
  discovered: "navigation.discovered.open",
  tracked: "navigation.tracked.open",
  metrics: "navigation.metrics.open",
  scraper: "navigation.workflows.open",
  settings: "navigation.settings.open",
};

export function TopNav({
  discoveredCount = 0,
  trackedCount = 0,
  scheduleControl,
}: TopNavProps) {
  const { view, setView } = useUIStore();

  return (
    <header className="flex flex-col w-full bg-[var(--color-paper)] shrink-0 select-none">
      <div className="flex flex-col sm:flex-row items-stretch justify-between min-h-10 px-3 gap-1 sm:gap-2 border-b border-[var(--color-ink)]">
        {/* Left: Tabs */}
        <div className="flex items-stretch gap-1 min-w-0 flex-1 overflow-x-auto">
          <nav className="flex items-stretch gap-1">
            {NAV_TABS.map((tab) => {
              const isActive = view === tab.id;
              const count = tab.id === "discovered" ? discoveredCount : tab.id === "tracked" ? trackedCount : null;

              return (
                <Tooltip
                  key={tab.id}
                  content={TAB_TOOLTIPS[tab.id as NavTabId]}
                  position="bottom"
                  className="flex items-stretch"
                >
                  <button
                    data-control-action={TAB_ACTIONS[tab.id as NavTabId]}
                    onClick={() => setView(tab.id as NavTabId)}
                    className={`px-2 sm:px-3 min-h-8 my-1.5 flex items-center justify-center font-mono text-xs font-bold tracking-wider transition-none cursor-pointer border ${
                      isActive
                        ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                        : "bg-transparent text-[var(--color-ink-mute)] border-transparent hover:text-[var(--color-ink)] hover:border-[var(--color-ink)]"
                    }`}
                  >
                    {tab.label}{count !== null && <span className="hidden sm:inline ml-1">({count})</span>}
                  </button>
                </Tooltip>
              );
            })}
          </nav>
        </div>

        {/* Right: Status Indicator / Next Run / Scanning & Notification Bell */}
        <div className="flex items-center justify-end gap-3 pb-1 sm:pb-0 font-mono text-xs text-[var(--color-ink)] shrink-0">
          {scheduleControl}

          <div className="h-4 w-px bg-[var(--color-ink-mute)]/30" />

          {/* External Terminal Console Launcher */}
          <Tooltip content="Open Live System Console in Terminal.app">
            <IconButton data-control-action="console.open" label="Open Terminal Console" id="terminal-console-btn" onClick={() => api.openTerminalConsole()}>
              <Icon name="terminal" />
            </IconButton>
          </Tooltip>

          <NotificationBell />
        </div>
      </div>
    </header>
  );
}
