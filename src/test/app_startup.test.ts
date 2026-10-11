import { describe, it, expect, beforeEach } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { useUIStore } from "../lib/store";
import { TopNav } from "../components/layout/TopNav";
import { Footer } from "../components/layout/Footer";
import fs from "fs";
import path from "path";

describe("Application Startup & Entrypoint Smoke Suite", () => {
  beforeEach(() => {
    useUIStore.setState({
      view: "discovered",
      isScanning: false,
      unreadNotificationCount: 0,
      isNotificationOpen: false,
    });
  });

  it("verifies index.html entrypoint integrity", () => {
    const indexPath = path.resolve(process.cwd(), "index.html");
    expect(fs.existsSync(indexPath)).toBe(true);

    const html = fs.readFileSync(indexPath, "utf-8");
    expect(html).toContain('<div id="root"></div>');
    expect(html).toContain('src="/src/main.tsx"');
    expect(html).toContain("<title>SCRAPE</title>");
  });

  it("verifies package.json lifecycle scripts for startup and build", () => {
    const pkgPath = path.resolve(process.cwd(), "package.json");
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));

    expect(pkg.scripts.dev).toBe("vite");
    expect(pkg.scripts.build).toBe("tsc && vite build");
    expect(pkg.scripts["build:sidecar"]).toBeTruthy();
    expect(pkg.scripts.test).toBeTruthy();
  });

  it("renders top application chrome (TopNav) cleanly on initial startup", () => {
    const markup = renderToStaticMarkup(React.createElement(TopNav));

    // Must render branding and all navigation tabs
    expect(markup).toContain("SCRAPE");
    expect(markup).toContain("DISCOVERED");
    expect(markup).toContain("TRACKED");
    expect(markup).toContain("METRICS");
    expect(markup).toContain("SCRAPER");
    expect(markup).toContain("SETTINGS");

    // Must render telemetry console launcher and notification bell
    expect(markup).toContain('id="terminal-console-btn"');
    expect(markup).toContain('id="notification-bell-btn"');
  });

  it("renders application status bar (Footer) cleanly on initial startup", () => {
    const markup = renderToStaticMarkup(React.createElement(Footer));

    expect(markup).toContain("OPEN");
    expect(markup).toContain("APPLY");
    expect(markup).toContain("TAB");
    expect(markup).toContain("SEARCH");
  });

  it("boots and switches between all application tabs without state corruption", () => {
    const store = useUIStore.getState();
    const tabs = ["discovered", "tracked", "metrics", "scraper", "settings"] as const;

    for (const tab of tabs) {
      store.setView(tab);
      expect(useUIStore.getState().view).toBe(tab);
    }
  });
});
