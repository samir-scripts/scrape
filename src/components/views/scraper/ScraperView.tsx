import { FiltersSettings } from "../settings/FiltersSettings";
import type { Settings } from "../../../bindings/Settings";
import { ScheduleSettings } from "../settings/ScheduleSettings";
import { SourcesSettings } from "../settings/SourcesSettings";

interface ScraperViewProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function ScraperView({ settings, onUpdateSetting }: ScraperViewProps) {
  return (
    <div className="flex-1 w-full h-full overflow-y-auto p-6 bg-[var(--color-paper)]">
      <details className="border border-[var(--color-ink)] p-3 mb-5">
        <summary className="font-mono cursor-pointer text-sm">
          Automatic scheduling and Run all
        </summary>
        <div className="pt-4">
          <ScheduleSettings
            settings={settings}
            onUpdateSetting={onUpdateSetting}
          />
        </div>
      </details>
      <details className="border border-[var(--color-ink)] p-3 mb-5"><summary className="font-mono cursor-pointer text-sm">Global matching defaults</summary><div className="pt-4"><FiltersSettings settings={settings} onUpdateSetting={onUpdateSetting} /></div></details>
      <SourcesSettings settings={settings} />
    </div>
  );
}
