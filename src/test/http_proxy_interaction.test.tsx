// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { api, DEFAULT_SETTINGS } from "../lib/api";
const native = vi.hoisted(() => ({ enabled: true, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, isTauri: () => native.enabled }));
afterEach(() => { cleanup(); vi.clearAllMocks(); native.enabled = true; });

it("saves write-only proxy credentials, clears fields, preserves status on failure and clears routing", async () => {
  let fail = false;
  native.invoke.mockImplementation(async (command: string) => {
    if (command === "get_adzuna_credential_status") return { configured: false };
    if (command === "get_http_proxy_status" || command === "clear_http_proxy") return { configured: false, server: null, authenticated: false };
    if (command === "set_http_proxy") {
      if (fail) throw new Error("do not display fixture-password");
      return { configured: true, server: "http://proxy.example:8080", authenticated: true };
    }
    throw new Error(`Unexpected command ${command}`);
  });
  const user = userEvent.setup();
  render(<SystemSettings settings={DEFAULT_SETTINGS} onUpdateSetting={vi.fn()} />);
  const panel = within(screen.getByRole("region", { name: "HTTP proxy" }));
  await user.type(panel.getByLabelText("Proxy endpoint"), "http://proxy.example:8080");
  await user.type(panel.getByLabelText(/Proxy username/), "fixture-user");
  await user.type(panel.getByLabelText(/Proxy password/), "fixture-password");
  await user.click(panel.getByRole("button", { name: "Save proxy" }));
  await waitFor(() => expect(panel.getByLabelText(/Proxy password/)).toHaveProperty("value", ""));
  expect(panel.getByLabelText(/Proxy username/)).toHaveProperty("value", "");
  expect(panel.getByLabelText("Proxy endpoint")).toHaveProperty("value", "");
  expect(native.invoke).toHaveBeenCalledWith("set_http_proxy", { server: "http://proxy.example:8080", username: "fixture-user", password: "fixture-password" });
  expect(panel.getByText(/Configured: http/)).toBeTruthy();
  fail = true;
  await user.type(panel.getByLabelText("Proxy endpoint"), "invalid");
  await user.click(panel.getByRole("button", { name: "Save proxy" }));
  await panel.findByText(/previous configuration is unchanged/);
  expect(panel.queryByText(/do not display/)).toBeNull();
  expect(panel.getByText(/Configured: http/)).toBeTruthy();
  await user.click(panel.getByRole("button", { name: "Clear proxy" }));
  await panel.findByText("No proxy configured.");
  expect(native.invoke).toHaveBeenCalledWith("clear_http_proxy");
});

it("does not configure or claim routing in demo mode", async () => {
  native.enabled = false;
  render(<SystemSettings settings={DEFAULT_SETTINGS} onUpdateSetting={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Save proxy" })).toHaveProperty("disabled", true);
  expect(screen.getByRole("button", { name: "Clear proxy" })).toHaveProperty("disabled", true);
  expect(screen.getByText(/Demo\/recorded mode does not configure proxy routing/)).toBeTruthy();
  await expect(api.setHttpProxy("http://proxy.example:8080")).rejects.toThrow(/desktop app/);
  await expect(api.clearHttpProxy()).rejects.toThrow(/desktop app/);
  expect(await api.getHttpProxyStatus()).toEqual({ configured: false, server: null, authenticated: false });
  expect(native.invoke).not.toHaveBeenCalled();
});
