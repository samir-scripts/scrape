import { Icon } from "../../ui/Icon";
import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import type { Settings } from "../../../bindings/Settings";
import type { HttpProxyStatus } from "../../../bindings/HttpProxyStatus";
import { Input } from "../../ui/Input";
import { api } from "../../../lib/api";
import { formatDateTime } from "../../../lib/formatters";
import { Toggle } from "../../ui/Toggle";
import { Button } from "../../ui/Button";
import { Rule } from "../../ui/Rule";
import { openPath } from "@tauri-apps/plugin-opener";
import { Select } from "../../ui/Select";
import { formatDate } from "../../../lib/formatters";

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
  const [adzunaAppId, setAdzunaAppId] = useState("");
  const [adzunaAppKey, setAdzunaAppKey] = useState("");
  const [adzunaConfigured, setAdzunaConfigured] = useState(false);
  const [adzunaBusy, setAdzunaBusy] = useState(false);
  const [adzunaStatus, setAdzunaStatus] = useState<string | null>(null);
  const [proxy, setProxy] = useState<HttpProxyStatus>({ configured: false, server: null, authenticated: false });
  const [proxyServer, setProxyServer] = useState("");
  const [proxyUsername, setProxyUsername] = useState("");
  const [proxyPassword, setProxyPassword] = useState("");
  const [proxyBusy, setProxyBusy] = useState(false);
  const [proxyMessage, setProxyMessage] = useState<string | null>(null);
  useEffect(() => {
    void api.getHttpProxyStatus().then(setProxy).catch(() => setProxyMessage("Unable to read proxy status."));
  }, []);
  const changeProxy = async (clear: boolean) => {
    setProxyBusy(true);
    setProxyMessage(null);
    try {
      setProxy(clear ? await api.clearHttpProxy() : await api.setHttpProxy(proxyServer, proxyUsername || null, proxyPassword || null));
      setProxyServer("");
      setProxyUsername("");
      setProxyPassword("");
      setProxyMessage(clear ? "Proxy cleared. New requests use direct routing." : "Proxy saved in the operating system secure store.");
    } catch {
      // Do not render transport/keyring errors that might echo credentials.
      setProxyMessage("Could not update proxy configuration. Check the endpoint and credential pair; the previous configuration is unchanged.");
    } finally {
      setProxyBusy(false);
    }
  };

  useEffect(() => {
    if (!isTauri()) return;
    void api.getAdzunaCredentialStatus().then(({ configured }) => {
      setAdzunaConfigured(configured);
    }).catch((error: unknown) => {
      setAdzunaStatus(error instanceof Error ? error.message : "Unable to read secure-store status.");
    });
  }, []);

  const handleSaveAdzuna = async () => {
    setAdzunaBusy(true);
    setAdzunaStatus(null);
    try {
      const result = await api.setAdzunaCredentials(adzunaAppId, adzunaAppKey);
      setAdzunaConfigured(result.configured);
      setAdzunaAppId("");
      setAdzunaAppKey("");
      setAdzunaStatus("Credentials saved in the operating system secure store. Enable Adzuna separately in a workflow.");
    } catch (error) {
      setAdzunaStatus(error instanceof Error ? error.message : "Could not save credentials.");
    } finally {
      setAdzunaBusy(false);
    }
  };

  const handleClearAdzuna = async () => {
    setAdzunaBusy(true);
    setAdzunaStatus(null);
    try {
      const result = await api.clearAdzunaCredentials();
      setAdzunaConfigured(result.configured);
      setAdzunaAppId("");
      setAdzunaAppKey("");
      setAdzunaStatus("Adzuna credentials cleared.");
    } catch (error) {
      setAdzunaStatus(error instanceof Error ? error.message : "Could not clear credentials.");
    } finally {
      setAdzunaBusy(false);
    }
  };

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

      <section className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30" aria-label="Automatic scraping">
        <h3 className="font-mono text-xs font-bold uppercase tracking-wider">Automatic scraping</h3>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">Pause or resume scheduled workflow runs. Manual runs remain available while paused.</p>
        <Toggle
          actionId="settings.system.automatic_scraping"
          checked={settings.scraper_enabled}
          onChange={(enabled) => onUpdateSetting("scraper.enabled", enabled)}
          label="AUTOMATIC SCRAPING ENABLED"
        />
      </section>

      <section
        className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30"
        aria-label="Web scraping & crawler"
      >
        <h3 className="font-mono text-xs font-bold uppercase tracking-wider">
          Web Scraping & Crawler (Dev Build)
        </h3>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">
          Configure crawl rules, natural request pacing, and robots policy for research and public web scraping.
        </p>

        <Toggle
          actionId="settings.system.respect_robots"
          checked={settings.scraper_respect_robots ?? false}
          onChange={(checked) => onUpdateSetting("scraper.respect_robots", checked)}
          label="RESPECT ROBOTS.TXT"
        />
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">
          {settings.scraper_respect_robots
            ? "Robots.txt is respected. Pages disallowed by robots policy will be rejected."
            : "Robots.txt compliance is disabled (dev/research mode). The crawler will fetch target career pages without robots restrictions."}
        </p>

        <div className="grid grid-cols-3 gap-4 pt-2">
          <label className="grid gap-1 text-xs text-[var(--color-ink-mute)]">
            DEFAULT MAX JOBS PER RUN
            <input
              data-control-action="settings.system.max_jobs"
              type="number"
              min={1}
              max={10000}
              value={settings.scraper_max_jobs_per_run ?? 20}
              onChange={(e) => onUpdateSetting("scraper.max_jobs_per_run", Number(e.target.value))}
              className="h-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 text-[var(--color-ink)] font-mono text-xs"
            />
          </label>
          <label className="grid gap-1 text-xs text-[var(--color-ink-mute)]">
            BASE REQUEST DELAY (MS)
            <input
              data-control-action="settings.system.request_delay"
              type="number"
              min={0}
              max={10000}
              value={settings.scraper_per_host_delay_ms ?? 750}
              onChange={(e) => onUpdateSetting("scraper.per_host_delay_ms", Number(e.target.value))}
              className="h-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 text-[var(--color-ink)] font-mono text-xs"
            />
          </label>
          <label className="grid gap-1 text-xs text-[var(--color-ink-mute)]">
            RANDOMIZED JITTER (MS)
            <input
              data-control-action="settings.system.request_jitter"
              type="number"
              min={0}
              max={10000}
              value={settings.scraper_jitter_ms ?? 500}
              onChange={(e) => onUpdateSetting("scraper.jitter_ms", Number(e.target.value))}
              className="h-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 text-[var(--color-ink)] font-mono text-xs"
            />
          </label>
        </div>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">
          Natural limits: between consecutive requests to the same host, the scraper sleeps for base delay + random jitter to emulate realistic browsing patterns.
        </p>
      </section>

      <section className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30" aria-label="Job providers">
        <h3 className="font-mono text-xs font-bold uppercase tracking-wider">Job providers / Adzuna</h3>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">Credentials stay in the operating system secure store. They are injected only for Adzuna execution; saving credentials does not enable a workflow source.</p>
        <p role="status" className="font-mono text-xs">{adzunaConfigured ? "Adzuna credentials configured." : "Adzuna setup required."}</p>
        <label className="grid gap-2 text-xs text-[var(--color-ink-mute)]">
          App ID
          <input data-control-action="settings.system.adzuna_app_id" autoComplete="off" value={adzunaAppId} onChange={(event) => setAdzunaAppId(event.target.value)} className="h-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 text-[var(--color-ink)]" />
        </label>
        <label className="grid gap-2 text-xs text-[var(--color-ink-mute)]">
          App key
          <input data-control-action="settings.system.adzuna_app_key" type="password" autoComplete="new-password" value={adzunaAppKey} onChange={(event) => setAdzunaAppKey(event.target.value)} className="h-9 border border-[var(--color-ink)] bg-[var(--color-paper)] px-2 text-[var(--color-ink)]" />
        </label>
        <div className="flex flex-wrap gap-2">
          <Button data-control-action="settings.system.save_adzuna" disabled={adzunaBusy || !adzunaAppId.trim() || !adzunaAppKey.trim() || !isTauri()} onClick={handleSaveAdzuna}>Save credentials</Button>
          <Button data-control-action="settings.system.clear_adzuna" variant="ghost" disabled={adzunaBusy || !adzunaConfigured || !isTauri()} onClick={handleClearAdzuna}>Clear credentials</Button>
        </div>
        {adzunaStatus && <p role="status" className="font-mono text-xs">{adzunaStatus}</p>}
      </section>

      <section aria-label="HTTP proxy" className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <h3 className="font-mono text-xs font-bold uppercase tracking-wider">Network / HTTP proxy</h3>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">Supply your own static HTTP(S) forward proxy; none is bundled. Routes built-in HTTP and app-launched Chromium only, with no direct fallback when configured. A proxy does not bypass robots, login walls or access restrictions.</p>
        <p className="font-mono text-xs text-[var(--color-ink-mute)]">Basic credentials sent to an HTTP proxy are not transport-encrypted. Prefer an HTTPS proxy or a trusted local network.</p>
        <p className="font-mono text-xs">{!isTauri() ? "Desktop app required. Demo/recorded mode does not configure proxy routing." : proxy.configured ? `Configured: ${proxy.server}${proxy.authenticated ? " (authenticated)" : ""}` : "No proxy configured."}</p>
        <label className="grid gap-2 text-xs">Proxy endpoint
          <Input data-control-action="settings.system.proxy_endpoint" autoComplete="off" placeholder="http://proxy.example:8080" value={proxyServer} onChange={event => setProxyServer(event.target.value)} />
        </label>
        <label className="grid gap-2 text-xs">Proxy username (optional, write-only)
          <Input data-control-action="settings.system.proxy_username" type="password" autoComplete="new-password" value={proxyUsername} onChange={event => setProxyUsername(event.target.value)} />
        </label>
        <label className="grid gap-2 text-xs">Proxy password (optional, write-only)
          <Input data-control-action="settings.system.proxy_password" type="password" autoComplete="new-password" value={proxyPassword} onChange={event => setProxyPassword(event.target.value)} />
        </label>
        <p className="text-xs text-[var(--color-ink-mute)]">Use an endpoint with scheme, host and port, without embedded credentials. Supply both username and password or neither. Saved credentials are never loaded into these fields.</p>
        <div className="flex gap-2">
          <Button data-control-action="settings.system.save_proxy" disabled={proxyBusy || !isTauri() || !proxyServer.trim()} onClick={() => void changeProxy(false)}>Save proxy</Button>
          <Button data-control-action="settings.system.clear_proxy" variant="ghost" disabled={proxyBusy || !isTauri() || !proxy.configured} onClick={() => void changeProxy(true)}>Clear proxy</Button>
        </div>
        {proxyMessage && <p role="alert" className="font-mono text-xs">{proxyMessage}</p>}
      </section>

      {/* Locale & Timezone Info */}
      <div className="flex flex-col gap-2 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono text-xs">
        <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          ENVIRONMENT
        </span>
        <div className="grid grid-cols-3 gap-4 pt-1">
          <div>
            <span className="text-xs text-[var(--color-ink-mute)] block">
              SYSTEM LOCALE
            </span>
            <span className="font-bold">{settings.system_locale}</span>
          </div>
          <div>
            <span className="text-xs text-[var(--color-ink-mute)] block">
              SYSTEM TIMEZONE
            </span>
            <span className="font-bold">{settings.system_timezone}</span>
          </div>
          <div>
            <span className="text-xs text-[var(--color-ink-mute)] block">
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
          <span className="text-xs text-[var(--color-ink-mute)]">
            PREVIEW: {formatDateTime(new Date(), { timeFormat: settings.system_time_format, dateFormat: settings.system_date_format })}
          </span>
        </div>

        <div className="grid grid-cols-1 gap-4 pt-1 sm:grid-cols-2">
          <label className="grid gap-2 text-xs text-[var(--color-ink-mute)]">
            TIME FORMAT
            <Select actionId="settings.system.time_format" aria-label="Time format" value={settings.system_time_format}
              options={[
                { value: "12h", label: "12-hour · 3:25 PM" },
                { value: "24h", label: "24-hour · 15:25" },
              ]}
              onChange={(fmt) => onUpdateSetting("system.time_format", fmt)} />
          </label>

          <label className="grid gap-2 text-xs text-[var(--color-ink-mute)]">
            DATE FORMAT
            <Select actionId="settings.system.date_format" aria-label="Date format" value={settings.system_date_format}
              options={(["MM/DD/YY", "DD/MM/YY", "MM/DD/YYYY", "DD/MM/YYYY", "YYYY-MM-DD"] as const).map((fmt) => ({
                value: fmt,
                label: `${fmt} · ${formatDate(new Date(2031, 10, 23), fmt)}`,
              }))}
              onChange={(fmt) => onUpdateSetting("system.date_format", fmt)} />
          </label>
        </div>
      </div>

      <Rule />

      {/* OS & Tray Integration Toggles */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          actionId="settings.system.launch_at_login"
          checked={settings.app_launch_at_login}
          onChange={(checked) =>
            onUpdateSetting("app.launch_at_login", checked)
          }
          label="LAUNCH AT SYSTEM LOGIN"
        />

        <Toggle
          actionId="settings.system.start_hidden"
          checked={settings.app_start_hidden}
          onChange={(checked) => onUpdateSetting("app.start_hidden", checked)}
          label="START MINIMIZED TO TRAY"
        />

        <Toggle
          actionId="settings.system.notifications"
          checked={settings.notifications_enabled}
          onChange={(checked) =>
            onUpdateSetting("notifications.enabled", checked)
          }
          label="DESKTOP NOTIFICATIONS"
        />

        <p className="text-sm">Job removal always asks whether to block future discovery. Restore saved jobs in Removed &amp; blocked jobs.</p>
      </div>

      <Rule />

      {/* Software Updates & Releases */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30 font-mono">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)] text-xs">
          SOFTWARE UPDATES & RELEASES
        </label>

        <Toggle
          actionId="settings.system.auto_update"
          checked={settings.app_auto_update ?? true}
          onChange={(checked) => onUpdateSetting("app.auto_update", checked)}
          label="CHECK FOR UPDATES AUTOMATICALLY ON STARTUP"
        />

        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-[var(--color-ink-mute)] font-mono">
            UPDATE CHANNEL
          </span>
          <div className="flex max-w-sm border border-[var(--color-ink)]">
            <button
              data-control-action="settings.system.channel_stable"
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
              data-control-action="settings.system.channel_nightly"
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
          <div className="flex items-center justify-between text-xs text-[var(--color-ink-mute)]">
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
              data-control-action="settings.system.check_updates"
              variant="default"
              disabled={isCheckingUpdate}
              onClick={handleCheckUpdate}
            >
              {isCheckingUpdate ? "CHECKING FOR UPDATES…" : "CHECK FOR UPDATES NOW"}
            </Button>
            {updateCheckStatus && (
              <span className="text-xs font-bold text-[var(--color-ink)]">
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
          <Button data-control-action="settings.system.export_json" variant="default" onClick={handleExportJson}>
            EXPORT TO JSON
          </Button>
          <Button data-control-action="settings.system.export_csv" variant="default" onClick={handleExportCsv}>
            EXPORT TO CSV
          </Button>
          <Button data-control-action="settings.system.open_folder" variant="ghost" onClick={handleOpenDataFolder}>
            OPEN PROJECT FOLDER <Icon name="external" size="sm" />
          </Button>
        </div>
      </div>
    </div>
  );
}
