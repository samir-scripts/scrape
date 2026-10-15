import type { Settings } from "../../../bindings/Settings";
import { SourcesSettings } from "../settings/SourcesSettings";
interface ScraperViewProps { settings: Settings; onUpdateSetting: (key: string, value: unknown) => void; }
export function ScraperView({ settings, onUpdateSetting }: ScraperViewProps) {
  return <div className="flex-1 w-full h-full overflow-y-auto p-4 sm:p-6 bg-[var(--color-paper)]">
    <SourcesSettings settings={settings} onUpdateSetting={onUpdateSetting} />
  </div>;
}
