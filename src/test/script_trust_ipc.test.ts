import { afterEach, expect, it, vi } from "vitest";
const native = vi.hoisted(() => ({ invoke: vi.fn(), enabled: true }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, isTauri: () => native.enabled }));
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import { importWorkflowYaml } from "../lib/workflowImport";
import { sourceToWorkflowSpec, workflowSpecToYaml } from "../lib/yamlWorkflow";

afterEach(() => { vi.clearAllMocks(); native.enabled = true; });

it("routes YAML imports through backend quarantine and passes import origin on save", async () => {
  const spec = sourceToWorkflowSpec(SAMPLE_SWE_SOURCE);
  native.invoke.mockResolvedValueOnce(spec).mockResolvedValueOnce(SAMPLE_SWE_SOURCE);
  expect(await importWorkflowYaml(workflowSpecToYaml(spec))).toEqual(spec);
  expect(native.invoke).toHaveBeenNthCalledWith(1, "import_workflow", { spec });
  await api.saveWorkflow(spec, "fixture", true);
  expect(native.invoke).toHaveBeenNthCalledWith(2, "save_workflow", { spec, id: "fixture", imported: true });
});

it("sends explicit confirmation and the reviewed fingerprint without inferring authority", async () => {
  native.invoke.mockResolvedValue(SAMPLE_SWE_SOURCE);
  await api.trustWorkflowScript("fixture", "reviewed-hash", false);
  expect(native.invoke).toHaveBeenCalledWith("trust_workflow_script", { sourceId: "fixture", expectedHash: "reviewed-hash", confirmed: false });
  native.enabled = false;
  await expect(api.trustWorkflowScript("fixture", "reviewed-hash", true)).rejects.toThrow("desktop application");
  expect(native.invoke).toHaveBeenCalledTimes(1);
});
