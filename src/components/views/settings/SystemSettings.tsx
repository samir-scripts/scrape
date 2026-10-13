import { useState } from "react";
import type { Settings } from "../../../bindings/Settings";
import { api } from "../../../lib/api";
import { formatDateTime } from "../../../lib/formatters";
import { Toggle } from "../../ui/Toggle";
import { Button } from "../../ui/Button";
import { Rule } from "../../ui/Rule";
import { openPath } from "@tauri-apps/plugin-opener";
import { isTauri } from "@tauri-apps/api/core";

interface SystemSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function SystemSettings({
  settings,
  onUpdateSetting,
}: SystemSettingsProps) {
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateCheckStatus, setUpdateCheckStatus] = useState<string | null>(null);

  const handleCheckUpdate = async () => {
    try {
      setIsCheckingUpdate(true);
      setUpdateCheckStatus("QUERYING RELEASE ENDPOINT…");
      const res = await api.checkAppUpdate();
      if (res.available) {
        setUpdateCheckStatus(`UPDATE AVAILABLE: v${res.latest_version || "NEW"}`);
      } else if (res.error) {
        setUpdateCheckStatus(`INFO: ${res.error}`);
      } else {
        setUpdateCheckStatus("APPLICATION IS UP TO DATE");
      }
    } catch (e) {
      setUpdateCheckStatus(e instanceof Error ? e.message : "UPDATE CHECK FAILED");
    } finally {
      setIsCheckingUpdate(false);
    }
  };
  const handleOpenDataFolder = async () => {
    try {
      if (isTauri()) {
        await openPath(".");
      } else {
        console.info("Opening project data folder is only available in desktop application.");
      }
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
          SYSTEM
        </h2>
      </div>

      <Rule />

      {/* Locale & Timezone Info */}
      <div className="flex flex-col gap-2 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono text-xs">
        <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          ENVIRONMENT
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

      {/* Regional & Time Format */}
      <div className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono text-xs">
        <div className="flex justify-between items-center border-b border-[var(--color-ink)]/30 pb-2">
          <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
            REGIONAL & TIME FORMAT
          </span>
          <span className="text-[10px] text-[var(--color-ink-mute)]">
            PREVIEW: {formatDateTime(new Date(), { timeFormat: settings.system_time_format, dateFormat: settings.system_date_format })}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-4 pt-1">
          <div>
            <span className="text-[11px] text-[var(--color-ink-mute)] block mb-1">
              TIME FORMAT
            </span>
            <div className="flex border border-[var(--color-ink)]">
              <button
                type="button"
                onClick={() => onUpdateSetting("system.time_format", "12h")}
                className={`flex-1 py-1 text-center font-bold text-xs cursor-pointer ${
                  settings.system_time_format === "12h"
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                    : "hover:bg-[var(--color-paper-2)]"
                }`}
              >
                12-HOUR (AM/PM)
              </button>
              <button
                type="button"
                onClick={() => onUpdateSetting("system.time_format", "24h")}
                className={`flex-1 py-1 text-center font-bold text-xs cursor-pointer ${
                  settings.system_time_format === "24h"
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                    : "hover:bg-[var(--color-paper-2)]"
                }`}
              >
                24-HOUR
              </button>
            </div>
          </div>

          <div>
            <span className="text-[11px] text-[var(--color-ink-mute)] block mb-1">
              DATE FORMAT
            </span>
            <div className="flex border border-[var(--color-ink)]">
              {(["MM/DD/YY", "DD/MM/YY", "MM/DD/YYYY", "DD/MM/YYYY", "YYYY-MM-DD"] as const).map((fmt) => (
                <button
                  key={fmt}
                  type="button"
                  onClick={() => onUpdateSetting("system.date_format", fmt)}
                  className={`flex-1 py-1 text-center font-bold text-[10px] cursor-pointer ${
                    settings.system_date_format === fmt
                      ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                      : "hover:bg-[var(--color-paper-2)]"
                  }`}
                >
                  {fmt}
                </button>
              ))}
            </div>
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
        />

        <Toggle
          checked={settings.app_start_hidden}
          onChange={(checked) => onUpdateSetting("app.start_hidden", checked)}
          label="START MINIMIZED TO TRAY"
        />

        <Toggle
          checked={settings.notifications_enabled}
          onChange={(checked) =>
            onUpdateSetting("notifications.enabled", checked)
          }
          label="DESKTOP NOTIFICATIONS"
        />

        <Toggle
          checked={settings.app_confirm_job_deletion}
          onChange={(checked) =>
            onUpdateSetting("app.confirm_job_deletion", checked)
          }
          label="CONFIRM BEFORE DELETING JOBS"
          description="PROMPT FOR CONFIRMATION BEFORE DELETING JOBS FROM DISCOVERED OR TRACKED"
        />
      </div>

      <Rule />

      {/* Software Updates & Releases */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)] text-xs">
          SOFTWARE UPDATES & RELEASES
        </label>

        <Toggle
          checked={settings.app_auto_update ?? true}
          onChange={(checked) => onUpdateSetting("app.auto_update", checked)}
          label="CHECK FOR UPDATES AUTOMATICALLY ON STARTUP"
        />

        <div className="flex flex-col gap-1.5">
          <span className="text-[11px] text-[var(--color-ink-mute)] font-mono">
            UPDATE CHANNEL
          </span>
          <div className="flex max-w-sm border border-[var(--color-ink)]">
            <button
              type="button"
              onClick={() => onUpdateSetting("app.update_channel", "stable")}
              className={`flex-1 py-1 text-center font-bold text-xs cursor-pointer ${
                (settings.app_update_channel || "stable") === "stable"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "hover:bg-[var(--color-paper-2)] text-[var(--color-ink)]"
              }`}
            >
              STABLE (RECOMMENDED)
            </button>
            <button
              type="button"
              onClick={() => onUpdateSetting("app.update_channel", "nightly")}
              className={`flex-1 py-1 text-center font-bold text-xs cursor-pointer ${
                settings.app_update_channel === "nightly"
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "hover:bg-[var(--color-paper-2)] text-[var(--color-ink)]"
              }`}
            >
              NIGHTLY
            </button>
          </div>
        </div>

        <div className="flex flex-col gap-2 pt-2 border-t border-[var(--color-ink-mute)]/30 text-xs">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-ink-mute)]">
            <span>CURRENT VERSION: v0.1.0</span>
            <span>
              LAST CHECKED:{" "}
              {settings.app_last_update_check
                ? formatDateTime(settings.app_last_update_check, settings)
                : "NEVER"}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="default"
              disabled={isCheckingUpdate}
              onClick={handleCheckUpdate}
            >
              {isCheckingUpdate ? "CHECKING FOR UPDATES…" : "CHECK FOR UPDATES NOW"}
            </Button>
            {updateCheckStatus && (
              <span className="text-[11px] font-bold text-[var(--color-ink)]">
                {updateCheckStatus}
              </span>
            )}
          </div>
        </div>
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
