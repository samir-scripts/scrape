import { useState } from "react";
import { FiltersSettings } from "../settings/FiltersSettings";
import type { Settings } from "../../../bindings/Settings";
import { ScheduleSettings } from "../settings/ScheduleSettings";
import { SourcesSettings } from "../settings/SourcesSettings";
import { Modal } from "../../ui/Modal";
interface ScraperViewProps { settings: Settings; onUpdateSetting: (key: string, value: unknown) => void; }
export function ScraperView({ settings, onUpdateSetting }: ScraperViewProps) {
  const [globalOpen, setGlobalOpen] = useState(false);
  return <div className="flex-1 w-full h-full overflow-y-auto p-4 sm:p-6 bg-[var(--color-paper)]">
    <SourcesSettings settings={settings} onOpenSettings={() => setGlobalOpen(true)} />
    <Modal isOpen={globalOpen} onClose={() => setGlobalOpen(false)} title="Scraper settings" width="max-w-[800px]">
      {globalOpen && <div className="grid gap-5"><p>Global defaults apply only where a workflow inherits them. Per-workflow overrides are configured in Add workflow or Edit.</p>
        <details open><summary className="cursor-pointer min-h-9">Global automatic scheduling</summary><ScheduleSettings settings={settings} onUpdateSetting={onUpdateSetting} /></details>
        <details><summary className="cursor-pointer min-h-9">Global matching defaults</summary><FiltersSettings settings={settings} onUpdateSetting={onUpdateSetting} /></details>
      </div>}
    </Modal>
  </div>;
}
