import type { Settings } from "../../../bindings/Settings";
import { api } from "../../../lib/api";
import { Toggle } from "../../ui/Toggle";
import { Button } from "../../ui/Button";
import { Rule } from "../../ui/Rule";
import { openPath } from "@tauri-apps/plugin-opener";

interface SystemSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function SystemSettings({
  settings,
  onUpdateSetting,
}: SystemSettingsProps) {
  const handleOpenDataFolder = async () => {
    try {
      // In macOS, Scrape SQLite DB is at ~/Library/Application Support/com.scrape.dev or similar appDataDir
      // If path is not known, we can request opener to open the app support directory or current dir
      await openPath(".");
    } catch (e) {
      console.error("Failed to open data folder:", e);
    }
  };

  const handleExportJson = async () => {
    try {
      const res = await api.listJobs({ limit: 5000 });
      const dataStr =
        "data:text/json;charset=utf-8," +
        encodeURIComponent(JSON.stringify(res.items, null, 2));
      const downloadAnchor = document.createElement("a");
      downloadAnchor.setAttribute("href", dataStr);
      downloadAnchor.setAttribute(
        "download",
        `scrape_jobs_export_${new Date().toISOString().slice(0, 10)}.json`
      );
      document.body.appendChild(downloadAnchor);
      downloadAnchor.click();
      downloadAnchor.remove();
    } catch (e) {
      console.error("Failed to export JSON:", e);
    }
  };

  const handleExportCsv = async () => {
    try {
      const res = await api.listJobs({ limit: 5000 });
      const headers = [
        "id",
        "title",
        "company",
        "role_type",
        "location",
        "status",
        "url",
        "date_posted",
        "date_added",
        "notes",
      ];
      const rows = res.items.map((j) => [
        `"${j.id}"`,
        `"${j.title.replace(/"/g, '""')}"`,
        `"${j.company.replace(/"/g, '""')}"`,
        `"${j.role_type}"`,
        `"${(j.location || "").replace(/"/g, '""')}"`,
        `"${j.status}"`,
        `"${j.url}"`,
        `"${j.date_posted || ""}"`,
        `"${j.date_added}"`,
        `"${(j.notes || "").replace(/"/g, '""')}"`,
      ]);

      const csvContent =
        "data:text/csv;charset=utf-8," +
        encodeURIComponent(
          [headers.join(","), ...rows.map((r) => r.join(","))].join("\n")
        );
      const downloadAnchor = document.createElement("a");
      downloadAnchor.setAttribute("href", csvContent);
      downloadAnchor.setAttribute(
        "download",
        `scrape_jobs_export_${new Date().toISOString().slice(0, 10)}.csv`
      );
      document.body.appendChild(downloadAnchor);
      downloadAnchor.click();
      downloadAnchor.remove();
    } catch (e) {
      console.error("Failed to export CSV:", e);
    }
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl select-none">
      <div>
        <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
          SYSTEM CONFIGURATION & OS INTEGRATION
        </h2>
        <p className="text-xs font-mono text-[var(--color-ink-mute)]">
          MANAGE LOCALE, OS AUTOSTART, NOTIFICATIONS, AND LOCAL EXPORTS
        </p>
      </div>

      <Rule />

      {/* Locale & Timezone Info */}
      <div className="flex flex-col gap-2 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono text-xs">
        <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          ENVIRONMENT & TIMEZONE DETECTION
        </span>
        <div className="grid grid-cols-3 gap-4 pt-1">
          <div>
            <span className="text-[11px] text-[var(--color-ink-mute)] block">
              SYSTEM LOCALE
            </span>
            <span className="font-bold">{settings.system_locale}</span>
          </div>
          <div>
            <span className="text-[11px] text-[var(--color-ink-mute)] block">
              SYSTEM TIMEZONE
            </span>
            <span className="font-bold">{settings.system_timezone}</span>
          </div>
          <div>
            <span className="text-[11px] text-[var(--color-ink-mute)] block">
              WEEK START DAY
            </span>
            <span className="font-bold uppercase">
              {settings.system_week_start}
            </span>
          </div>
        </div>
      </div>

      <Rule />

      {/* OS & Tray Integration Toggles */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          checked={settings.app_launch_at_login}
          onChange={(checked) =>
            onUpdateSetting("app.launch_at_login", checked)
          }
          label="LAUNCH AT SYSTEM LOGIN"
          description="Registers Scrape with the operating system login items (macOS LaunchAgent / Windows Run)."
        />

        <Toggle
          checked={settings.app_start_hidden}
          onChange={(checked) => onUpdateSetting("app.start_hidden", checked)}
          label="START MINIMIZED TO TRAY"
          description="Launches silently into the menu bar / system tray without opening the main window."
        />

        <Toggle
          checked={settings.notifications_enabled}
          onChange={(checked) =>
            onUpdateSetting("notifications.enabled", checked)
          }
          label="OS DESKTOP NOTIFICATIONS"
          description="Displays native desktop notifications when new matching job postings are discovered."
        />
      </div>

      <Rule />

      {/* Backup and Data Export */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          LOCAL DATA & EXPORT ENGINE
        </label>
        <div className="flex items-center gap-3">
          <Button variant="default" onClick={handleExportJson}>
            EXPORT TO JSON
          </Button>
          <Button variant="default" onClick={handleExportCsv}>
            EXPORT TO CSV
          </Button>
          <Button variant="ghost" onClick={handleOpenDataFolder}>
            OPEN PROJECT FOLDER ↗
          </Button>
        </div>
      </div>
    </div>
  );
}
