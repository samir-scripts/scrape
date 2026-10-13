import { describe, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke, isTauri: () => true }));
import { api } from "../lib/api";
describe("native error messages", () => {
  it("preserves the actionable backend path instead of rendering an object", async () => {
    mocks.invoke.mockRejectedValueOnce({ kind: "validation", message: "Validation error: filters.query: expected a value" });
    await expect(api.validateWorkflow({})).rejects.toThrow("filters.query: expected a value");
    mocks.invoke.mockRejectedValueOnce({ kind: "database", message: "Database error: disk is full" });
    await expect(api.removeJob("fixture", true)).rejects.toMatchObject({ message: "Database error: disk is full", kind: "database" });
  });
});
