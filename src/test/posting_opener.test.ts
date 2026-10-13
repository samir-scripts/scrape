import { afterEach, describe, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ open: vi.fn(), native: vi.fn(() => true) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: mocks.open }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: mocks.native }));
import { openPostingUrl } from "../lib/postingUrl";
afterEach(() => vi.clearAllMocks());
describe("posting opener boundary", () => {
  it("passes the stored custom provider destination unchanged", async () => {
    const destination = "https://careers.example.com/apply?gh_jid=42&utm_campaign=provider#apply";
    await openPostingUrl(destination);
    expect(mocks.open).toHaveBeenCalledExactlyOnceWith(destination);
  });
  it("rejects unsafe legacy schemes and reports native opener failure", async () => {
    await expect(openPostingUrl("javascript:alert(1)")).rejects.toThrow("HTTP or HTTPS");
    expect(mocks.open).not.toHaveBeenCalled();
    mocks.open.mockRejectedValueOnce(new Error("Opening failed"));
    await expect(openPostingUrl("https://example.com/jobs/42")).rejects.toThrow("Opening failed");
  });
});
