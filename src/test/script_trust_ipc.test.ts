import { afterEach, expect, it, vi } from "vitest";
const native = vi.hoisted(() => ({ invoke: vi.fn(), enabled: true }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, isTauri: () => native.enabled }));
import { api, SAMPLE_ENGINEERING_SOURCE } from "../lib/api";
import { importWorkflowYaml } from "../lib/workflowImport";
import { sourceToWorkflowSpec, workflowSpecToYaml } from "../lib/yamlWorkflow";

afterEach(() => { vi.clearAllMocks(); native.enabled = true; });

it("routes YAML imports through backend quarantine and passes import origin on save", async () => {
  const spec = sourceToWorkflowSpec(SAMPLE_ENGINEERING_SOURCE);
  native.invoke.mockResolvedValueOnce(spec).mockResolvedValueOnce(SAMPLE_ENGINEERING_SOURCE);
  expect(await importWorkflowYaml(workflowSpecToYaml(spec))).toEqual(spec);
  expect(native.invoke).toHaveBeenNthCalledWith(1, "import_workflow", { spec });
  await api.saveWorkflow(spec, "fixture", true);
  expect(native.invoke).toHaveBeenNthCalledWith(2, "save_workflow", { spec, id: "fixture", imported: true });
});

it("sends explicit confirmation and the reviewed fingerprint without inferring authority", async () => {
  native.invoke.mockResolvedValue(SAMPLE_ENGINEERING_SOURCE);
  await api.trustWorkflowScript("fixture", "reviewed-hash", false);
  expect(native.invoke).toHaveBeenCalledWith("trust_workflow_script", { sourceId: "fixture", expectedHash: "reviewed-hash", confirmed: false });
  native.enabled = false;
  await expect(api.trustWorkflowScript("fixture", "reviewed-hash", true)).rejects.toThrow("desktop application");
  expect(native.invoke).toHaveBeenCalledTimes(1);
});

it("routes browser runtime, session lifecycle, and extractor test commands to desktop IPC", async () => {
  const runtimeStatus = { available: true, executable_path: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", version: "130.0", message: null };
  const sessionSummary = { id: "session-1", label: "Workday", domain: "workday.example.com", status: "ready" as const, created_at: "2026-10-17T00:00:00Z", updated_at: "2026-10-17T00:00:00Z", last_verified_at: "2026-10-17T00:00:00Z" };
  const testResult = { samples: [], field_match_counts: { title: 0 }, warnings: [], termination: "exhausted" as const };
  native.invoke
    .mockResolvedValueOnce(runtimeStatus)
    .mockResolvedValueOnce([sessionSummary])
    .mockResolvedValueOnce(sessionSummary)
    .mockResolvedValueOnce(sessionSummary)
    .mockResolvedValueOnce(undefined)
    .mockResolvedValueOnce(testResult);

  expect(await api.detectBrowserRuntime()).toEqual(runtimeStatus);
  expect(await api.listBrowserSessions()).toEqual([sessionSummary]);
  expect(await api.openBrowserSession("Workday", "https://workday.example.com/login")).toEqual(sessionSummary);
  expect(await api.saveBrowserSession("session-1")).toEqual(sessionSummary);
  await api.clearBrowserSession("session-1");
  expect(await api.testExtractor({ kind: "selector", url: "https://workday.example.com/jobs", mode: "browser" })).toEqual(testResult);

  expect(native.invoke).toHaveBeenNthCalledWith(1, "detect_browser_runtime");
  expect(native.invoke).toHaveBeenNthCalledWith(2, "list_browser_sessions");
  expect(native.invoke).toHaveBeenNthCalledWith(3, "open_browser_session", { label: "Workday", loginUrl: "https://workday.example.com/login" });
  expect(native.invoke).toHaveBeenNthCalledWith(4, "save_browser_session", { sessionId: "session-1" });
  expect(native.invoke).toHaveBeenNthCalledWith(5, "clear_browser_session", { sessionId: "session-1" });
  expect(native.invoke).toHaveBeenNthCalledWith(6, "test_extractor", { extractor: { kind: "selector", url: "https://workday.example.com/jobs", mode: "browser" } });
});
