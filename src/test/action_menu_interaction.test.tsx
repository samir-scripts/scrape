// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionMenu } from "../components/ui/ActionMenu";
import { Modal } from "../components/ui/Modal";

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function() { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function() { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("keeps menu commands themed, named, keyboard accessible, and inside the native dialog", async () => {
  const select = vi.fn();
  render(<Modal isOpen onClose={() => {}} title="Posting">
    <ActionMenu label="More actions" items={[
      { label: "Open history", onSelect: select },
      { label: "Delete posting", onSelect: vi.fn(), danger: true, dividerBefore: true },
    ]} />
  </Modal>);
  const trigger = screen.getByRole("button", { name: "More actions" });
  const user = userEvent.setup();
  trigger.focus(); await user.keyboard("{Enter}");
  const menu = screen.getByRole("menu");
  expect(screen.getByRole("dialog", { name: "Posting" }).contains(menu)).toBe(true);
  expect(screen.getByRole("menuitem", { name: "Delete posting" })).toBeTruthy();
  await user.keyboard("{Escape}");
  expect(screen.getByRole("dialog", { name: "Posting" })).toBeTruthy();
  await waitFor(() => expect(document.activeElement).toBe(trigger));
  await user.keyboard("{Enter}");
  await user.click(screen.getByRole("menuitem", { name: "Open history" }));
  expect(select).toHaveBeenCalledOnce();
  await waitFor(() => expect(document.activeElement).toBe(trigger));
});
