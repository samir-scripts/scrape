// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import { sourceToWorkflowSpec, validateWorkflowYaml, workflowSpecToYaml, normalizedEndpoint } from "../lib/yamlWorkflow";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const preview = (spec: WorkflowSpec): WorkflowPreview => ({ spec, branding: [], jobs: [], fetched: 3, matched: 0, rejected: 3, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: "2026-10-14T13:00:00Z" });
async function chooseMulti(user: UserEvent, label: string, text: string) {
  const input = screen.getByRole("combobox", { name: label });
  await user.click(input); await user.clear(input); await user.type(input, text);
  const options = screen.getAllByRole("option");
  const option = options.find(item => item.textContent?.includes(`Use “${text}”`)) ??
    options.find(item => item.textContent?.includes(text)) ?? options[0];
  await user.click(option);
}
async function toSchedule(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
  await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
  await user.click(screen.getByRole("button", { name: "Continue to Review" }));
}
describe("role-first workflow boundaries", () => {
  it.each([
    { roles: "Software Engineer", companies: "Cloudflare,Linear", date: "Past 12 hours", preset: "past_12h" },
    { roles: "Line Cook,Store Associate", companies: "", date: "Today (since local midnight)", preset: "today" },
  ])("creates $roles without a career URL, previews and saves without scraping", async ({ roles, companies, date, preset }) => {
    const user = userEvent.setup();
    const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
    const run = vi.spyOn(api, "runSourceNow");
    const detect = vi.spyOn(api, "detectSource");
    const extract = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => preview(spec));
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    expect(screen.queryByRole("textbox", { name: "Source URL or board identifier" })).toBeNull();
    for (const role of roles.split(",")) await chooseMulti(user, "Target roles", role);
    for (const company of companies.split(",").filter(Boolean)) await chooseMulti(user, "Companies (Optional)", company);
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
    await user.click(screen.getByRole("button", { name: "Date posted range", expanded: false }));
    await user.click(screen.getByRole("option", { name: date }));
    await toSchedule(user);
    expect((screen.getByRole("button", { name: "Save workflow" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    await user.click(screen.getByRole("button", { name: "Save workflow" }));
    await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    const spec = save.mock.calls[0][0];
    expect(spec.version).toBe(8);
    expect(spec.sources).toHaveLength(companies ? companies.split(",").length + 1 : 1);
    expect(spec.sources.filter(source => source.extractor.kind === "search")).toHaveLength(1);
    expect(spec.filters?.matching?.target_roles).toEqual(roles.split(","));
    expect(spec.filters?.matching?.companies).toEqual(companies ? companies.split(",") : []);
    expect(spec.filters?.matching?.posting_date).toMatchObject({ preset, last_hours: null, from: null, through: null, include_unknown: false });
    expect(extract.mock.calls[0][0]).toEqual(spec);
    expect(detect).not.toHaveBeenCalled(); expect(run).not.toHaveBeenCalled();
  });
  it("only auto-names drafts until a user supplies a name, including names containing jobs or at", async () => {
    const user = userEvent.setup();
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await chooseMulti(user, "Target roles", "Engineer");
    expect((screen.getByLabelText("Workflow name") as HTMLInputElement).value).toBe("Engineer jobs");
    await user.clear(screen.getByLabelText("Workflow name"));
    await user.type(screen.getByLabelText("Workflow name"), "My jobs at home");
    await chooseMulti(user, "Companies (Optional)", "Acme");
    await chooseMulti(user, "Target roles", "Intern");
    expect((screen.getByLabelText("Workflow name") as HTMLInputElement).value).toBe("My jobs at home");
    expect(screen.getByRole("button", { name: "Remove Engineer from Target roles" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove Intern from Target roles" })).toBeTruthy();
  });
  it("defaults to 20, invalidates preview on limit changes, rejects invalid drafts and saves YAML limit", async () => {
    const user = userEvent.setup();
    const extract = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => preview(spec));
    const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
    render(<AddSourceModal isOpen initialSource={SAMPLE_SWE_SOURCE} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
    const limit = screen.getByRole("textbox", { name: "Maximum matching jobs per run" });
    expect((limit as HTMLInputElement).value).toBe("20");
    await toSchedule(user);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    await user.click(screen.getByRole("button", { name: "Criteria" }));
    for (const invalid of ["0", "1.5", "10001", "abc"]) {
      await user.clear(limit); await user.type(limit, invalid);
      expect(limit.getAttribute("aria-invalid")).toBe("true");
      await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
      expect(screen.getByRole("button", { name: "Criteria", current: "step" })).toBeTruthy();
      expect(extract).toHaveBeenCalledTimes(1);
    }
    await user.clear(limit); await user.type(limit, "7");
    await user.click(screen.getByRole("button", { name: "Advanced" }));
    await user.click(screen.getByRole("button", { name: "Edit YAML" }));
    expect((screen.getByLabelText("Workflow YAML") as HTMLTextAreaElement).value).toContain("max_jobs_per_run: 7");
    await user.click(screen.getByRole("button", { name: "Apply YAML to form" }));
    await toSchedule(user);
    expect((screen.getByRole("button", { name: "Save workflow" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    await user.click(screen.getByRole("button", { name: "Save workflow" }));
    await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    expect(save.mock.calls[0][0].max_jobs_per_run).toBe(7);
  });
  it("preserves saved names, cents, arbitrary relative hours and legacy filters until edited", async () => {
    const user = userEvent.setup();
    const spec = sourceToWorkflowSpec(SAMPLE_SWE_SOURCE);
    spec.metadata.name = "My jobs at home";
    spec.filters = { ...spec.filters, matching: { ...spec.filters?.matching, target_roles: ["Engineer"], companies: ["Acme"], posting_date: { preset: null, last_hours: 37, from: null, through: null, timezone: "America/New_York", include_unknown: true }, compensation: { min_minor: 5025, max_minor: 6075, currency: "USD", interval: "hour", include_unknown: false } } } as WorkflowSpec["filters"];
    const extract = vi.spyOn(api, "previewWorkflow").mockImplementation(async input => preview(input));
    render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await chooseMulti(user, "Target roles", "Intern");
    expect((screen.getByLabelText("Workflow name") as HTMLInputElement).value).toBe("My jobs at home");
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
    expect((screen.getByLabelText("Last N hours") as HTMLInputElement).value).toBe("37");
    expect((screen.getByRole("textbox", { name: "Minimum per hour" }) as HTMLInputElement).value).toBe("50.25");
    await toSchedule(user);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    expect(extract.mock.calls[0][0].filters?.matching?.posting_date).toEqual(spec.filters?.matching?.posting_date);
    expect(extract.mock.calls[0][0].filters?.matching?.compensation).toEqual(spec.filters?.matching?.compensation);
    expect(extract.mock.calls[0][0].sources).toEqual(spec.sources);
    await user.click(screen.getByRole("button", { name: "Criteria" }));
    await user.click(screen.getByRole("button", { name: "Date posted range", expanded: false }));
    await user.click(screen.getByRole("option", { name: "Custom date range…" }));
    expect(screen.getByLabelText("Posted from")).toBeTruthy(); expect(screen.getByLabelText("Posted through")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Date posted range", expanded: false }));
    await user.click(screen.getByRole("option", { name: "Any time" }));
    await toSchedule(user);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await waitFor(() => expect(extract).toHaveBeenCalledTimes(2));
    expect(extract.mock.calls[1][0].filters?.matching?.posting_date).toBeNull();
  });
  it("ignores cancelled stale preview results and requires a new preview after cadence edits", async () => {
    const user = userEvent.setup();
    let finish!: (value: WorkflowPreview) => void;
    const pending = new Promise<WorkflowPreview>(resolve => { finish = resolve; });
    const extract = vi.spyOn(api, "previewWorkflow").mockImplementationOnce(() => pending).mockImplementation(async spec => preview(spec));
    const cancel = vi.spyOn(api, "cancelWorkflowPreview").mockResolvedValue(true);
    const settings = vi.spyOn(api, "updateSetting");
    render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, cadence: "global" }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" })); await toSchedule(user);
    await user.click(screen.getByRole("button", { name: "Schedule" }));
    expect(screen.getByRole("button", { name: "Schedule", expanded: false }).textContent).toContain("daily");
    await user.click(screen.getByRole("button", { name: "Continue to Review" }));
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await user.click(screen.getByRole("button", { name: "Cancel preview" }));
    finish(preview(extract.mock.calls[0][0]));
    await waitFor(() => expect(screen.queryByText(/3 examined/)).toBeNull());
    expect(cancel).toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    await user.click(screen.getByRole("button", { name: "Schedule" }));
    await user.click(screen.getByRole("button", { name: "Schedule", expanded: false }));
    expect(screen.queryByRole("option", { name: "global" })).toBeNull();
    await user.click(screen.getByRole("option", { name: "manual" }));
    expect(screen.getByRole("checkbox", { name: "Run this workflow when the app starts" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Continue to Review" }));
    expect((screen.getByRole("button", { name: "Save workflow" }) as HTMLButtonElement).disabled).toBe(true);
    expect(settings).not.toHaveBeenCalled();
  });
  it("re-detects an interrupted board edit and keeps the latest keywords", async () => {
    const user = userEvent.setup();
    let finish!: (value: { kind: string; identifier: string; name: string }) => void;
    const pending = new Promise<{ kind: string; identifier: string; name: string }>(resolve => { finish = resolve; });
    const result = { kind: "greenhouse", identifier: "acme-new", name: "Acme" };
    const detect = vi.spyOn(api, "detectSource").mockImplementationOnce(() => pending).mockResolvedValue(result);
    const extract = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => preview(spec));
    render(<AddSourceModal isOpen initialSource={SAMPLE_SWE_SOURCE} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.type(screen.getByRole("combobox", { name: "Any keywords" }), "Engineer");
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
    await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
    const url = screen.getByRole("textbox", { name: "Source URL or board identifier" });
    await user.clear(url); await user.type(url, "https://boards.greenhouse.io/acme");
    await user.click(screen.getByRole("button", { name: "Detect source 1" }));
    await user.clear(url); await user.type(url, "https://boards.greenhouse.io/acme-new");
    await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
    await user.click(screen.getByRole("button", { name: "Continue to Review" }));
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 examined/);
    expect(detect).toHaveBeenCalledTimes(2);
    expect(extract.mock.calls[0][0].sources[0].extractor.kind).toBe("greenhouse");
    expect(extract.mock.calls[0][0].filters?.matching?.keywords?.any).toEqual(["Engineer"]);
    finish({ kind: "lever", identifier: "stale", name: "Stale" });
    await waitFor(() => expect(screen.getByText(/3 examined/)).toBeTruthy());
    expect(screen.queryByText("Stale")).toBeNull();
  });
  it("keeps the populated workflow draft across mode switches and exposes Review as the final step", async () => {
    const user = userEvent.setup();
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    const advanced = screen.getByRole("button", { name: "Advanced" });
    await user.click(advanced);
    expect(advanced.getAttribute("aria-pressed")).toBe("true");
    await user.type(screen.getByRole("combobox", { name: "Any keywords" }), "distributed systems{Enter}");
    expect(screen.getByRole("button", { name: "Remove distributed systems from Any keywords" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Simple" }));
    expect(screen.getByRole("button", { name: "Simple" }).getAttribute("aria-pressed")).toBe("true");
    await user.click(screen.getByRole("button", { name: "Advanced" }));
    expect(screen.getByRole("button", { name: "Remove distributed systems from Any keywords" })).toBeTruthy();
    for (const step of ["Criteria", "Sources", "Schedule", "Review"]) {
      await user.click(screen.getByRole("button", { name: `Continue to ${step}` }));
    }
    expect(screen.getByRole("button", { name: "Review", current: "step" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Save workflow" }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("roundtrips version8 search and role/company/date rules without silent drops", () => {
    const spec = sourceToWorkflowSpec(SAMPLE_SWE_SOURCE); spec.version = 8;
    spec.sources = [{ id: "search", label: "Public listings", enabled: true, extractor: { kind: "search", url: "" } }];
    spec.filters = { matching: { ...spec.filters?.matching, target_roles: ["Line Cook"], companies: ["Acme"], posting_date: { preset: "past_week", last_hours: null, from: null, through: null, timezone: "UTC", include_unknown: false } } } as WorkflowSpec["filters"];
    spec.filters!.matching!.experience_match_mode = "required_minimum";
    const result = validateWorkflowYaml(workflowSpecToYaml(spec));
    expect(result.errors).toEqual([]); expect(result.parsedSpec).toEqual(spec);
    expect(normalizedEndpoint("search", "")).toBe(normalizedEndpoint("search", "default"));
    expect(normalizedEndpoint("search", "")).toBe(normalizedEndpoint("search", "themuse"));
    spec.filters!.matching!.posting_date!.last_hours = 168;
    expect(validateWorkflowYaml(workflowSpecToYaml(spec)).errors.some(error => /cannot be combined/.test(error.message))).toBe(true);
  });
});
