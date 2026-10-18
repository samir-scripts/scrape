import { afterEach, expect, it, vi } from "vitest";
const native = vi.hoisted(() => ({ invoke: vi.fn(), enabled: true }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, isTauri: () => native.enabled }));
import { api } from "../lib/api";
afterEach(() => { vi.clearAllMocks(); native.enabled = true; });
it("requests authoritative descriptions without transforming compatibility restrictions", async () => {
  const filters = { include_keywords: ["S_ft%"], locations: ["Remote", "Austin"], query: "title LIKE '%Intern%'", policy: { title: "%Engineer%" } };
  const descriptions = [{ field: "locations", explanation: "Original location text OR terms" }];
  native.invoke.mockResolvedValue(descriptions);
  expect(await api.describeWorkflowFilters(filters)).toEqual(descriptions);
  expect(native.invoke).toHaveBeenCalledWith("describe_workflow_filters", { filters });
  expect(filters.locations).toEqual(["Remote", "Austin"]);
  native.enabled = false;
  await expect(api.describeWorkflowFilters(filters)).rejects.toThrow("desktop validator");
  expect(native.invoke).toHaveBeenCalledTimes(1);
});
