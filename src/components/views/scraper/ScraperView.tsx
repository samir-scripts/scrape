import type { Settings } from "../../../bindings/Settings";
import { useUIStore, type ScraperCategory } from "../../../lib/store";
import { SourcesSettings } from "../settings/SourcesSettings";
import { ScheduleSettings } from "../settings/ScheduleSettings";
import { FiltersSettings } from "../settings/FiltersSettings";
import { LogsSettings } from "../settings/LogsSettings";

interface ScraperViewProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

const CATEGORIES: { id: ScraperCategory; label: string }[] = [
  { id: "sources", label: "SOURCES" },
  { id: "schedule", label: "SCHEDULE" },
  { id: "filters", label: "FILTERS" },
  { id: "logs", label: "LOGS" },
];

export function ScraperView({
  settings,
  onUpdateSetting,
}: ScraperViewProps) {
  const { scraperCategory, setScraperCategory } = useUIStore();

  return (
    <div className="flex-1 w-full h-full flex overflow-hidden bg-[var(--color-paper)]">
      {/* Left Sidebar: Sub-categories */}
      <aside className="w-56 shrink-0 h-full border-r border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 flex flex-col py-3 select-none">
        <div className="px-4 pb-2 mb-2 border-b border-[var(--color-ink)]">
          <span className="font-mono font-black text-xs uppercase tracking-wider text-[var(--color-ink)]">
            SCRAPER HUB
          </span>
        </div>

        <nav className="flex flex-col">
          {CATEGORIES.map((cat) => {
            const isActive = scraperCategory === cat.id;

            return (
              <button
                key={cat.id}
                onClick={() => setScraperCategory(cat.id)}
                className={`px-4 py-2.5 flex items-center justify-between text-xs font-mono uppercase tracking-wider cursor-pointer border-b border-[var(--color-ink)]/20 transition-none ${
                  isActive
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                    : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <span>{cat.label}</span>
                {isActive && (
                  <span className="text-[10px] text-[var(--color-paper)]">
                    ▸
                  </span>
                )}
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Right Content: Active Category Panel */}
      <main className="flex-1 h-full overflow-y-auto p-6 bg-[var(--color-paper)]">
        {scraperCategory === "sources" && <SourcesSettings />}
        {scraperCategory === "schedule" && (
          <ScheduleSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
        {scraperCategory === "filters" && (
          <FiltersSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
        {scraperCategory === "logs" && <LogsSettings />}
      </main>
    </div>
  );
}
