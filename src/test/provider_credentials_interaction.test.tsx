// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { DEFAULT_SETTINGS } from "../lib/api";

const native = vi.hoisted(() => ({ enabled: true, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, isTauri: () => native.enabled }));

afterEach(() => { cleanup(); vi.clearAllMocks(); native.enabled = true; });

it("stores write-only Adzuna credentials, clears form fields, and never enables a workflow", async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === "get_http_proxy_status") return { configured: false, server: null, authenticated: false };
    if (command === "get_adzuna_credential_status") return { configured: false };
    if (command === "set_adzuna_credentials") return { configured: true };
    throw new Error(`Unexpected command ${command}`);
  });
  const user = userEvent.setup();
  render(<SystemSettings settings={DEFAULT_SETTINGS} onUpdateSetting={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole("status").textContent).toContain("setup required"));
  await user.type(screen.getByLabelText("App ID"), "fixture-app-id");
  await user.type(screen.getByLabelText("App key"), "fixture-app-key");
  await user.click(screen.getByRole("button", { name: "Save credentials" }));
  await waitFor(() => expect(screen.getByLabelText("App ID")).toHaveProperty("value", ""));
  expect(screen.getByLabelText("App key")).toHaveProperty("value", "");
  expect(native.invoke).toHaveBeenCalledWith("set_adzuna_credentials", {
    appId: "fixture-app-id",
    appKey: "fixture-app-key",
  });
  expect(screen.getByText(/does not enable a workflow source/)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Clear credentials" })).toHaveProperty("disabled", false);
});

it("clears configured credentials and keeps browser-mode setup visibly unavailable", async () => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === "get_http_proxy_status") return { configured: false, server: null, authenticated: false };
    if (command === "get_adzuna_credential_status") return { configured: true };
    if (command === "clear_adzuna_credentials") return { configured: false };
    throw new Error(`Unexpected command ${command}`);
  });
  const user = userEvent.setup();
  const { rerender } = render(<SystemSettings settings={DEFAULT_SETTINGS} onUpdateSetting={vi.fn()} />);
  const clear = await screen.findByRole("button", { name: "Clear credentials" });
  await user.click(clear);
  await waitFor(() => expect(screen.getByText("Adzuna credentials cleared.")).toBeTruthy());
  expect(native.invoke).toHaveBeenCalledWith("clear_adzuna_credentials");

  native.enabled = false;
  rerender(<SystemSettings settings={DEFAULT_SETTINGS} onUpdateSetting={vi.fn()} />);
  await user.type(screen.getByLabelText("App ID"), "not-persisted");
  await user.type(screen.getByLabelText("App key"), "not-persisted");
  expect(screen.getByRole("button", { name: "Save credentials" })).toHaveProperty("disabled", true);
  expect(native.invoke).toHaveBeenCalledTimes(3);
});
