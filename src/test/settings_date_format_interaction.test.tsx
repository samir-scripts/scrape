// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { useState } from "react";
import { DEFAULT_SETTINGS } from "../lib/api";
import { formatDate } from "../lib/formatters";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("selects a readable date example and updates the labeled time radio group", async () => {
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

  await user.click(screen.getByRole("radio", { name: "24-hour" }));
  expect(updates).toContainEqual(["system.time_format", "24h"]);
  expect((screen.getByRole("radio", { name: "24-hour" }) as HTMLInputElement).checked).toBe(true);
  expect(screen.getByRole("radiogroup", { name: "Time format" })).toBeTruthy();
});
