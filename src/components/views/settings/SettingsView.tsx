import type { Settings } from "../../../bindings/Settings";
import { useUIStore, type SettingsCategory } from "../../../lib/store";
import { ScheduleSettings } from "./ScheduleSettings";
import { SourcesSettings } from "./SourcesSettings";
import { FiltersSettings } from "./FiltersSettings";
import { LogsSettings } from "./LogsSettings";
import { AppearanceSettings } from "./AppearanceSettings";
import { SystemSettings } from "./SystemSettings";

interface SettingsViewProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

const CATEGORIES: { id: SettingsCategory; label: string; desc: string }[] = [
  { id: "schedule", label: "SCHEDULE", desc: "TIMERS & CRON" },
  { id: "sources", label: "SOURCES", desc: "TARGET BOARDS" },
  { id: "filters", label: "SCRAPER", desc: "INGEST PRE-FILTERS" },
  { id: "logs", label: "LOGS", desc: "EXECUTION AUDIT" },
  { id: "appearance", label: "APPEARANCE", desc: "THEMES & DITHER" },
  { id: "system", label: "SYSTEM", desc: "OS & EXPORTS" },
];

export function SettingsView({
  settings,
  onUpdateSetting,
}: SettingsViewProps) {
  const { settingsCategory, setSettingsCategory } = useUIStore();

  return (
    <div className="flex-1 w-full h-full flex overflow-hidden bg-[var(--color-paper)]">
      {/* Left Sidebar: Categories */}
      <aside className="w-64 shrink-0 h-full border-r border-[var(--color-ink)] bg-[var(--color-paper-2)]/20 flex flex-col py-3 select-none">
        <div className="px-4 pb-2 mb-2 border-b border-[var(--color-ink)]">
          <span className="font-mono font-black text-xs uppercase tracking-wider text-[var(--color-ink)]">
            SYSTEM SETUP
          </span>
        </div>

        <nav className="flex flex-col">
          {CATEGORIES.map((cat) => {
            const isActive = settingsCategory === cat.id;

            return (
              <button
                key={cat.id}
                onClick={() => setSettingsCategory(cat.id)}
                className={`px-4 py-2.5 flex items-center justify-between text-xs font-mono uppercase tracking-wider cursor-pointer border-b border-[var(--color-ink)]/20 transition-none ${
                  isActive
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] font-bold"
                    : "text-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                <span>{cat.label}</span>
                <span
                  className={`text-[10px] ${
                    isActive
                      ? "text-[var(--color-paper)] opacity-80"
                      : "text-[var(--color-ink-mute)]"
                  }`}
                >
                  {isActive ? "▸" : cat.desc}
                </span>
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Right Content: Active Category Panel */}
      <main className="flex-1 h-full overflow-y-auto p-6 bg-[var(--color-paper)]">
        {settingsCategory === "schedule" && (
          <ScheduleSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
        {settingsCategory === "sources" && <SourcesSettings />}
        {settingsCategory === "filters" && (
          <FiltersSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
        {settingsCategory === "logs" && <LogsSettings />}
        {settingsCategory === "appearance" && (
          <AppearanceSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
        {settingsCategory === "system" && (
          <SystemSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        )}
      </main>
    </div>
  );
}
