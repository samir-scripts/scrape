// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Select } from "../components/ui/Select";
import { Modal } from "../components/ui/Modal";

afterEach(cleanup);
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function() { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function() { this.removeAttribute("open"); };
});

it("supports keyboard opening, option selection, disabled options, Escape and focus restoration", async () => {
  let value = "alpha";
  function Harness() {
    const [current, setCurrent] = useState(value);
    return <Select aria-label="Choice" value={current} onChange={next => { value = next; setCurrent(next); }} options={[
      { value: "alpha", label: "Alpha" }, { value: "beta", label: "Beta", disabled: true }, { value: "charlie", label: "Charlie" },
    ]} />;
  }
  render(<Harness />);
  const trigger = screen.getByRole("button", { name: "Choice" });
  trigger.focus();
  const user = userEvent.setup();
  await user.keyboard("{ArrowDown}");
  await user.click(await screen.findByRole("option", { name: "Charlie" }));
  expect(value).toBe("charlie");
  expect(document.activeElement).toBe(trigger);
  await user.click(trigger);
  expect(screen.queryByRole("option", { name: "Beta" })).toBeTruthy();
  await user.click(screen.getByRole("option", { name: "Beta" }));
  expect(value).toBe("charlie");
  expect(screen.getByRole("option", { name: "Beta" }).getAttribute("aria-disabled")).toBe("true");
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("listbox")).toBeNull();
  expect(document.activeElement).toBe(trigger);
});

it("keeps its popup within a native dialog and lets the first Escape close only the popup", async () => {
  const user = userEvent.setup();
  render(<Modal isOpen onClose={() => {}} title="Settings">
    <Select aria-label="Choice" value="one" onChange={() => {}} options={[{ value: "one", label: "One" }, { value: "two", label: "Two" }]} />
  </Modal>);
  const dialog = screen.getByRole("dialog", { name: "Settings" });
  await user.click(screen.getByRole("button", { name: "Choice" }));
  const popup = screen.getByRole("listbox");
  expect(dialog.contains(popup)).toBe(true);
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("listbox")).toBeNull();
  expect(screen.getByRole("dialog", { name: "Settings" })).toBeTruthy();
});
