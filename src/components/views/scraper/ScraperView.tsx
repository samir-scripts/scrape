import type { Settings } from "../../../bindings/Settings";
import { SourcesSettings } from "../settings/SourcesSettings";
interface ScraperViewProps { settings: Settings; }
export function ScraperView({ settings }: ScraperViewProps) {
  return <div className="workflow-viewport flex-1 w-full h-full overflow-y-auto p-4 sm:p-6 bg-[var(--color-paper)]">
    <SourcesSettings settings={settings} />
  </div>;
}
