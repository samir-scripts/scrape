// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ResizableSplitPane } from "../components/layout/ResizableSplitPane";
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
it("provides a named keyboard splitter with clamped bounds and a reset preference", async () => {
  const write = vi.fn();
  vi.stubGlobal("localStorage", { getItem: () => null, setItem: write });
  render(<ResizableSplitPane primary={<p>Results</p>} secondary={({ reset }) => <button onClick={reset}>Reset width</button>} />);
  const separator = screen.getByRole("separator", { name: "Job results width" });
  expect(separator.getAttribute("aria-controls")).toBe(screen.getByRole("region", { name: "Job results" }).id);
  const user = userEvent.setup(); separator.focus(); await user.keyboard("{Home}");
  expect(separator.getAttribute("aria-valuenow")).toBe(separator.getAttribute("aria-valuemin"));
  await user.keyboard("{End}");
  expect(separator.getAttribute("aria-valuenow")).toBe(separator.getAttribute("aria-valuemax"));
  await user.click(screen.getByRole("button", { name: "Reset width" }));
  expect(write).toHaveBeenLastCalledWith("scrape:job_split:v1", '{"version":1,"ratio":0.6}');
});
