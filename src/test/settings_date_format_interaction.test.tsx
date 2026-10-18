// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { useState } from "react";
import { DEFAULT_SETTINGS } from "../lib/api";
import { formatDate } from "../lib/formatters";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("selects a readable date example and updates the labeled time select control", async () => {
  const updates: [string, unknown][] = [];
  function Harness() {
    const [settings, setSettings] = useState(DEFAULT_SETTINGS);
    return <SystemSettings settings={settings} onUpdateSetting={(key, value) => {
      updates.push([key, value]);
      setSettings(current => key === "system.date_format"
        ? { ...current, system_date_format: String(value) }
        : { ...current, system_time_format: String(value) });
    }} />;
  }
  render(<Harness />);
  const user = userEvent.setup();

  for (const format of ["MM/DD/YY", "DD/MM/YY", "MM/DD/YYYY", "DD/MM/YYYY", "YYYY-MM-DD"]) {
    await user.click(screen.getByRole("button", { name: "Date format" }));
    const example = `${format} · ${formatDate(new Date(2031, 10, 23), format)}`;
    await user.click(await screen.findByRole("option", { name: example }));
    expect(screen.getByRole("button", { name: "Date format" }).textContent).toContain(format);
  }
  expect(updates.filter(([key]) => key === "system.date_format").map(([, value]) => value)).toEqual([
    "MM/DD/YY", "DD/MM/YY", "MM/DD/YYYY", "DD/MM/YYYY", "YYYY-MM-DD",
  ]);

  await user.click(screen.getByRole("button", { name: "Time format" }));
  await user.click(await screen.findByRole("option", { name: "24-hour · 15:25" }));
  expect(updates).toContainEqual(["system.time_format", "24h"]);
  expect(screen.getByRole("button", { name: "Time format" }).textContent).toContain("24-hour · 15:25");
});

it("stores the application-wide automatic scraping pause in System settings", async () => {
  const update = vi.fn();
  render(<SystemSettings settings={{ ...DEFAULT_SETTINGS, scraper_enabled: false }} onUpdateSetting={update} />);
  const toggle = screen.getByRole("switch", { name: "AUTOMATIC SCRAPING ENABLED" });
  await userEvent.setup().click(toggle);
  expect(update).toHaveBeenCalledWith("scraper.enabled", true);
});
