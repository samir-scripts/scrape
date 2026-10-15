// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DiscoveredView } from "../components/views/discovered/DiscoveredView";
import { recordedJobs } from "../lib/recordedJobs";
import { useUIStore } from "../lib/store";

let width = 1280;
const observers: (() => void)[] = [];
const values = new Map<string, string>();
const write = vi.fn((key: string, value: string) => values.set(key, value));
beforeEach(() => {
  width = 1280; observers.length = 0; values.clear(); write.mockClear();
  useUIStore.setState({ selectedJobId: null, drawerJobId: null });
  vi.stubGlobal("localStorage", { getItem: (key: string) => values.get(key) ?? null, setItem: write });
  vi.stubGlobal("ResizeObserver", class { constructor(callback: () => void) { observers.push(callback); } observe() {} disconnect() {} });
  vi.stubGlobal("PointerEvent", class extends MouseEvent { pointerId: number; constructor(type: string, options: PointerEventInit = {}) { super(type, options); this.pointerId = options.pointerId ?? 1; } });
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(() => ({ width, height: 600, left: 0, right: width, top: 0, bottom: 600, x: 0, y: 0, toJSON: () => ({}) }));
  Object.defineProperty(HTMLElement.prototype, "setPointerCapture", { configurable: true, value: vi.fn() });
  Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", { configurable: true, value: vi.fn() });
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", { configurable: true, value: function(this: HTMLDialogElement) { this.setAttribute("open", ""); } });
  Object.defineProperty(HTMLDialogElement.prototype, "close", { configurable: true, value: function(this: HTMLDialogElement) { this.removeAttribute("open"); } });
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
const fixture = { ...recordedJobs()[0], id: "resize-job", status: "viewed" as const, notes: "Original note" };
function mount() {
  const notes = vi.fn().mockResolvedValue(undefined);
  const view = render(<DiscoveredView jobs={[fixture]} onUpdateStatus={vi.fn()} onUpdateNotes={notes} onOpenUrl={vi.fn()} onUndo={vi.fn()} />);
  return { ...view, notes };
}
it("resizes by keyboard, preserves the same draft/scroll node through narrow and wide layouts, and resets", async () => {
  const { notes } = mount(); const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: /^View / }));
  const separator = screen.getByRole("separator", { name: "Job results width" });
  const start = Number(separator.getAttribute("aria-valuenow"));
  separator.focus(); await user.keyboard("{ArrowLeft}");
  expect(Number(separator.getAttribute("aria-valuenow"))).toBeLessThan(start);
  await user.keyboard("{Home}"); expect(separator.getAttribute("aria-valuenow")).toBe(separator.getAttribute("aria-valuemin"));
  await user.keyboard("{End}"); expect(separator.getAttribute("aria-valuenow")).toBe(separator.getAttribute("aria-valuemax"));
  fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "Still editing" } });
  const input = screen.getByLabelText("Notes");
  const scroll = input.closest("[data-dialog-scroll]") as HTMLElement;
  scroll.scrollTop = 150;
  act(() => { width = 480; observers.forEach(callback => callback()); });
  expect(screen.getByRole("dialog", { name: "Job details" })).toBeTruthy();
  expect(screen.getByLabelText("Notes")).toBe(input);
  act(() => { width = 1280; observers.forEach(callback => callback()); });
  expect(screen.getByRole("complementary", { name: "Job details" })).toBeTruthy();
  expect(screen.getByLabelText("Notes")).toBe(input);
  expect((input as HTMLTextAreaElement).value).toBe("Still editing");
  expect(scroll.scrollTop).toBe(150);
  expect(notes).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Reset pane width" }));
  expect(Number(separator.getAttribute("aria-valuenow"))).toBe(60);
});
it("persists only completed pointer drags, cancels safely, and restores the preference on reopen", async () => {
  const view = mount(); const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: /^View / }));
  const separator = screen.getByRole("separator", { name: "Job results width" });
  const original = Number(separator.getAttribute("aria-valuenow"));
  write.mockClear();
  fireEvent.pointerDown(separator, { pointerId: 7, button: 0, clientX: 760 });
  fireEvent.pointerMove(separator, { pointerId: 7, clientX: 850 });
  expect(write).not.toHaveBeenCalled();
  expect(Number(separator.getAttribute("aria-valuenow"))).toBeGreaterThan(original);
  fireEvent.pointerCancel(separator, { pointerId: 7 });
  expect(Number(separator.getAttribute("aria-valuenow"))).toBe(original);
  expect(write).not.toHaveBeenCalled();
  fireEvent.pointerDown(separator, { pointerId: 8, button: 0, clientX: 760 });
  fireEvent.pointerMove(separator, { pointerId: 8, clientX: 820 });
  fireEvent.pointerUp(separator, { pointerId: 8, clientX: 820 });
  expect(write).toHaveBeenCalledTimes(1);
  const saved = Number(separator.getAttribute("aria-valuenow"));
  view.unmount(); mount();
  await user.click(screen.getByRole("button", { name: /^View / }));
  expect(Number(screen.getByRole("separator", { name: "Job results width" }).getAttribute("aria-valuenow"))).toBe(saved);
});
it.each(['broken JSON', '{"version":2,"ratio":0.8}', '{"version":1,"ratio":9}'])("uses a safe default for an unsupported preference: %s", async stored => {
  values.set("scrape:job_split:v1", stored);
  mount(); await userEvent.click(screen.getByRole("button", { name: /^View / }));
  expect(screen.getByRole("separator", { name: "Job results width" }).getAttribute("aria-valuenow")).toBe("60");
});
