// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createPortal } from "react-dom";
import { installControlActivity, recordControlAction } from "../lib/controlActivity";
import { Toggle } from "../components/ui/Toggle";
import { Select } from "../components/ui/Select";
import { Modal } from "../components/ui/Modal";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("records annotated activation and committed edits once without user content", async () => {
  const log = vi.spyOn(console, "info").mockImplementation(() => {});
  const stop = installControlActivity();
  expect(installControlActivity()).toBe(stop);
  render(<>
    <button data-control-action="workflow.delete.request">Secret workflow name</button>
    <input aria-label="secret credential" data-control-action="workflow.delete.cancel" />
    <button disabled data-control-action="workflow.delete.confirm">Disabled</button>
    {createPortal(<button data-control-action="workflow.delete.confirm">Portal action</button>, document.body)}
  </>);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Secret workflow name" }));
  await user.type(screen.getByLabelText("secret credential"), "sentinel-password");
  await user.tab();
  await user.click(screen.getByRole("button", { name: "Portal action" }));
  expect(log.mock.calls).toEqual([
    ["UI activate workflow.delete.request: Request workflow deletion"],
    ["UI change workflow.delete.cancel: Cancel workflow deletion"],
    ["UI activate workflow.delete.confirm: Confirm workflow deletion"],
  ]);
  expect(JSON.stringify(log.mock.calls)).not.toContain("sentinel-password");
  expect(JSON.stringify(log.mock.calls)).not.toContain("Secret workflow name");
  recordControlAction("sentinel-password", "activate");
  expect(log).toHaveBeenCalledTimes(3);
  stop();
});

it("does not duplicate checkbox change or keyboard activation", async () => {
  const log = vi.spyOn(console, "info").mockImplementation(() => {});
  const stop = installControlActivity();
  render(<><label><input type="checkbox" data-control-action="workflow.delete.request" />Choice</label><button data-control-action="workflow.delete.cancel">Cancel</button></>);
  const user = userEvent.setup();
  await user.click(screen.getByText("Choice"));
  await user.tab();
  await user.keyboard("{Enter}");
  expect(log.mock.calls).toEqual([
    ["UI change workflow.delete.request: Request workflow deletion"],
    ["UI activate workflow.delete.cancel: Cancel workflow deletion"],
  ]);
  stop();
});

it("records custom selection, keyboard switch, submit and modal dismissal without labels", async () => {
  HTMLDialogElement.prototype.showModal = function() { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function() { this.removeAttribute("open"); };
  const log = vi.spyOn(console, "info").mockImplementation(() => {});
  const stop = installControlActivity();
  const close = vi.fn();
  render(<Modal isOpen onClose={close} title="Private dialog name">
    <Toggle actionId="workflow.delete.request" checked={false} onChange={() => {}} label="Private switch" />
    <Select actionId="workflow.delete.confirm" aria-label="Private select" value="a" onChange={() => {}} options={[{ value: "a", label: "Initial" }, { value: "secret-value", label: "Secret option" }]} />
    <form data-control-action="workflow.delete.cancel" onSubmit={event => event.preventDefault()}>
      <button type="submit">Submit private form</button>
    </form>
  </Modal>);
  const user = userEvent.setup();
  screen.getByRole("switch").focus();
  await user.keyboard("{Enter}");
  await user.click(screen.getByRole("button", { name: "Private select" }));
  await user.click(await screen.findByRole("option", { name: "Secret option" }));
  await user.click(screen.getByRole("button", { name: "Submit private form" }));
  await user.keyboard("{Escape}");
  expect(close).toHaveBeenCalledTimes(1);
  const entries = log.mock.calls.map(([entry]) => String(entry));
  expect(entries.filter(entry => entry.startsWith("UI change workflow.delete.request"))).toHaveLength(1);
  expect(entries.filter(entry => entry.startsWith("UI change workflow.delete.confirm"))).toHaveLength(1);
  expect(entries.filter(entry => entry.startsWith("UI submit workflow.delete.cancel"))).toHaveLength(1);
  expect(entries.filter(entry => entry.startsWith("UI dismiss dialog.dismiss"))).toHaveLength(1);
  expect(entries.join(" ")).not.toMatch(/Private|Secret option|secret-value/);
  stop();
});

it("bounds the native queue and continues after a private transport failure", async () => {
  let release!: () => void;
  const first = new Promise<void>(resolve => { release = resolve; });
  const send = vi.fn().mockReturnValueOnce(first).mockRejectedValueOnce(new Error("sentinel-secret")).mockResolvedValue(undefined);
  vi.resetModules();
  vi.doMock("@tauri-apps/api/core", () => ({ invoke: send, isTauri: () => true }));
  const isolated = await import("../lib/controlActivity");
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  isolated.recordControlAction("workflow.delete.request", "activate");
  for (let index = 0; index < 1024; index++) isolated.recordControlAction("workflow.delete.cancel", "activate");
  expect(error).toHaveBeenCalledWith("Could not record application interaction");
  release();
  await waitFor(() => expect(send).toHaveBeenCalledTimes(1024));
  isolated.recordControlAction("workflow.delete.confirm", "activate");
  await waitFor(() => expect(send).toHaveBeenCalledTimes(1025));
  expect(send).toHaveBeenLastCalledWith("log_control_interaction", { action: "workflow.delete.confirm", kind: "activate" });
  expect(JSON.stringify(send.mock.calls)).not.toContain("sentinel-secret");
  expect(JSON.stringify(error.mock.calls)).not.toContain("sentinel-secret");
  vi.doUnmock("@tauri-apps/api/core");
});
