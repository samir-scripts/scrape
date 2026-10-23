// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import { useWorkflowDraft } from "../hooks/useWorkflowDraft";
import { WorkflowSections, emptyRules } from "../components/views/settings/WorkflowSections";
import { companyOptions, selectedCompanyId } from "../components/views/settings/CompanyPicker";
import { api } from "../lib/api";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });
beforeEach(() => {
  vi.spyOn(api, "searchRoleTitles").mockImplementation(async query =>
    query.toLowerCase().includes("software")
      ? [{ title: "Software Engineer", soc_code: "15-1252", priority_tier: 1 }]
      : [],
  );
});
function Editor({ initialSpec }: { initialSpec?: WorkflowSpec }) {
  const model = useWorkflowDraft({ isOpen: true, initialSpec, onSourceSaved: () => {} });
  return <><WorkflowSections draft={model.draft} change={model.change} advanced={false} section="Roles" setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} />
    <WorkflowSections draft={model.draft} change={model.change} advanced={false} section="Sources" selectCompany={model.selectCompany} setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} />
    {!initialSpec && <button onClick={() => model.change(draft => ({ ...draft, sources: [{ id: "source-1", enabled: true, label: "", extractor: { kind: "greenhouse", url: "" } }] }))}>Attach company board</button>}
    {model.pendingCompany && <section aria-label="Replace source"><button onClick={model.cancelCompany}>Keep editing</button><button onClick={model.confirmCompany}>Replace source</button></section>}
    <output aria-label="Current draft">{JSON.stringify(model.draft)}</output></>;
}
const readDraft = (): WorkflowSpec => JSON.parse(screen.getByLabelText("Current draft").textContent!);
async function choose(user: UserEvent, index: number, query: string) {
  const input = screen.getAllByRole("combobox", { name: "Search companies" })[index];
  await user.click(input); await user.clear(input); await user.type(input, query); await user.keyboard("{ArrowDown}{Enter}");
}
describe("company board attachment editing", () => {
  it("searches every token across provider/domain, commits by keyboard, restores Escape and keeps custom entry available", async () => {
    const user = userEvent.setup(); render(<Editor />);
    await user.click(screen.getByRole("button", { name: "Attach company board" }));
    await choose(user, 0, "cloud green");
    const draft = readDraft();
    expect(draft.metadata.name).toBe("New Job Workflow");
    expect(draft.version).toBe(8);
    expect(draft.filters?.matching?.experience_match_mode).toBe("required_minimum");
    expect(draft.sources[0].id).toBe("source-1");
    expect(selectedCompanyId(draft.sources[0])).toBe("cloudflare");
    const input = screen.getByRole("combobox", { name: "Search companies" });
    await user.clear(input); await user.type(input, "not-an-employer");
    expect(screen.getByText("No matches")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Enter a custom careers URL" })).toBeTruthy();
    await user.keyboard("{Escape}");
    expect((input as HTMLInputElement).value).toBe("Cloudflare");
    expect(readDraft()).toEqual(draft);
    await user.click(screen.getByRole("button", { name: "Enter a custom careers URL" }));
    expect(screen.getByRole("textbox", { name: "Source URL or board identifier" })).toBeTruthy();
    expect(selectedCompanyId({ ...draft.sources[0], extractor: { ...draft.sources[0].extractor, url: "https://boards.greenhouse.io/cloudflare-impostor" } })).toBeNull();
  });
  it("cancels replacement without mutation, then replaces only one source while preserving protected draft edits", async () => {
    const cloudflare = companyOptions.find(option => option.company.id === "cloudflare")!;
    const spec: WorkflowSpec = { version: 7, metadata: { name: "My edited workflow", enabled: false, company: "My company" },
      sources: [{ ...structuredClone(cloudflare.source), id: "keep-slot", enabled: false }, { ...structuredClone(cloudflare.source), id: "other-slot", extractor: { kind: "jsonld", url: "https://example.com/careers" } }],
      filters: { matching: { ...emptyRules(), experience_min: 3, experience_max: 5, new_grad: "exclude" } },
      schedule: { cadence: "manual", run_on_startup: false }, retention: { days: 90, policy: "untracked_only" }, max_jobs_per_run: 7 };
    const user = userEvent.setup(); render(<Editor initialSpec={spec} />);
    await choose(user, 0, "nvidia");
    expect(readDraft()).toEqual(spec);
    await user.click(within(screen.getByRole("region", { name: "Replace source" })).getByRole("button", { name: "Keep editing" }));
    expect(readDraft()).toEqual(spec);
    await choose(user, 0, "nvidia");
    await user.click(within(screen.getByRole("region", { name: "Replace source" })).getByRole("button", { name: "Replace source" }));
    const next = readDraft();
    expect(next.sources[0].id).toBe("keep-slot");
    expect(next.sources[0].enabled).toBe(false);
    expect(selectedCompanyId(next.sources[0])).toBe("nvidia");
    expect(next.sources[1]).toEqual(spec.sources[1]);
    expect(next.metadata).toEqual(spec.metadata);
    expect(next.filters).toEqual(spec.filters);
    expect(next.schedule).toEqual(spec.schedule);
    expect(next.retention).toEqual(spec.retention);
    expect(next.max_jobs_per_run).toBe(7);
  });
  it("does not overwrite an edited name or apply example matching when selecting a company", async () => {
    const user = userEvent.setup(); render(<Editor />);
    await user.click(screen.getByRole("button", { name: "Attach company board" }));
    await user.clear(screen.getByRole("textbox", { name: "Workflow name" }));
    await user.type(screen.getByRole("textbox", { name: "Workflow name" }), "Personal search");
    await choose(user, 0, "netflix");
    const draft = readDraft();
    expect(draft.metadata.name).toBe("Personal search");
    expect(draft.filters?.matching?.keywords).toBeNull();
    expect(draft.sources[0].enabled).toBe(false);
    expect(screen.getByText(/Source disabled until explicitly enabled/)).toBeTruthy();
  });
  it("adds literal role and company filters atomically with deduplicated preset boards", async () => {
    const user = userEvent.setup(); render(<Editor />);
    const roles = screen.getByRole("combobox", { name: "Target roles" });
    await user.type(roles, "software eng");
    await screen.findByRole("option", { name: /Software Engineer/ });
    await user.keyboard("{ArrowDown}{Enter}");
    const companies = screen.getByRole("combobox", { name: "Companies (Optional)" });
    await user.type(companies, "Cloudflare");
    await screen.findByRole("option", { name: /Cloudflare/ });
    await user.keyboard("{ArrowDown}{Enter}");
    await user.type(companies, "Linear");
    await screen.findByRole("option", { name: /Linear/ });
    await user.keyboard("{ArrowDown}{Enter}");
    const draft = readDraft();
    expect(draft.filters?.matching?.target_roles).toEqual(["Software Engineer"]);
    expect(draft.filters?.matching?.companies).toEqual(["Cloudflare", "Linear"]);
    expect(draft.sources.map(source => source.id)).toHaveLength(4);
    expect(draft.sources.slice(2).map(source => source.employer_hint)).toEqual(["Cloudflare", "Linear"]);
    expect(draft.sources[2].enabled).toBe(true);
    expect(draft.sources[3].enabled).toBe(true);
    const remove = screen.getByRole("button", { name: "Remove Cloudflare from Companies (Optional)" });
    await user.click(remove);
    expect(readDraft().filters?.matching?.companies).toEqual(["Linear"]);
    expect(readDraft().sources).toHaveLength(4);
  });
  it("deduplicates a customized existing board without overwriting source identity or configuration", async () => {
    const cloudflare = companyOptions.find(option => option.company.id === "cloudflare")!;
    const customized = { ...structuredClone(cloudflare.source), id: "hand-authored-cloudflare", label: "My Cloudflare source", enabled: false };
    const spec: WorkflowSpec = {
      version: 8, metadata: { name: "Keep configured source", enabled: false },
      sources: [{ id: "public-search", label: "Public search", enabled: true, extractor: { kind: "search", url: "" } }, customized],
      filters: { matching: emptyRules() },
    };
    const user = userEvent.setup(); render(<Editor initialSpec={spec} />);
    const companies = screen.getByRole("combobox", { name: "Companies (Optional)" });
    await user.type(companies, "Cloudflare");
    await screen.findByRole("option", { name: /Cloudflare/ });
    await user.keyboard("{ArrowDown}{Enter}");
    expect(readDraft().filters?.matching?.companies).toEqual(["Cloudflare"]);
    expect(readDraft().sources).toEqual(spec.sources);
    await user.click(screen.getByRole("button", { name: "Remove Cloudflare from Companies (Optional)" }));
    expect(readDraft().sources).toEqual(spec.sources);
    await user.type(companies, "Cloudflare");
    await screen.findByRole("option", { name: /Cloudflare/ });
    await user.keyboard("{ArrowDown}{Enter}");
    expect(readDraft().filters?.matching?.companies).toEqual(["Cloudflare"]);
    expect(readDraft().sources).toEqual(spec.sources);
  });
  it("keeps substring company names custom and attaches non-ready catalog boards disabled", async () => {
    const user = userEvent.setup(); render(<Editor />);
    const companies = screen.getByRole("combobox", { name: "Companies (Optional)" });
    await user.type(companies, "Cloud");
    await screen.findByRole("option", { name: "Use “Cloud”" });
    await user.click(screen.getByRole("option", { name: "Use “Cloud”" }));
    expect(readDraft().sources).toHaveLength(2);
    await user.type(companies, "Google");
    await screen.findByRole("option", { name: /Google/ });
    await user.keyboard("{ArrowDown}{Enter}");
    const draft = readDraft();
    expect(draft.filters?.matching?.companies).toEqual(["Cloud", "Google"]);
    expect(draft.sources).toHaveLength(3);
    expect(draft.sources[1].extractor.kind).toBe("search");
    expect(draft.sources[2].enabled).toBe(false);
  });
  it("preserves imported company filters without implicit attachment and offers explicit reattachment", async () => {
    const cloudflare = companyOptions.find(option => option.company.id === "cloudflare")!;
    const spec: WorkflowSpec = {
      version: 8,
      metadata: { name: "Imported company filter", enabled: true },
      sources: [{ id: "public-search", label: "Public search", enabled: true, extractor: { kind: "search", url: "" } }],
      filters: { matching: { ...emptyRules(), companies: ["Cloudflare"] } },
    };
    const user = userEvent.setup(); render(<Editor initialSpec={spec} />);
    expect(readDraft()).toEqual(spec);
    expect(screen.getByRole("button", { name: "Attach career board for Cloudflare" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Attach career board for Cloudflare" }));
    const draft = readDraft();
    expect(draft.sources).toHaveLength(2);
    expect(draft.sources[0]).toEqual(spec.sources[0]);
    expect(selectedCompanyId(draft.sources[1])).toBe(cloudflare.company.id);
    expect(draft.sources[1].enabled).toBe(true);
    expect(screen.queryByRole("button", { name: "Attach career board for Cloudflare" })).toBeNull();
    expect(readDraft().sources).toHaveLength(2);
  });
  it("selects a locally indexed employer as a literal filter without attaching an unsupported board", async () => {
    const search = vi.spyOn(api, "searchCompanies").mockResolvedValue([
      { name: "Dollar General", catalog_company_id: null, jobs_seen: 8 },
      { name: "Dollar General Logistics", catalog_company_id: null, jobs_seen: 2 },
    ]);
    const user = userEvent.setup();
    render(<Editor />);
    const companies = screen.getByRole("combobox", { name: "Companies (Optional)" });
    await user.type(companies, "Dollar General");
    const exact = await screen.findByRole("option", { name: /Dollar General.*8 saved postings/ });
    expect(exact.textContent).toContain("employer filter only; no verified career source");
    await user.click(exact);
    expect(search).toHaveBeenCalledWith("Dollar General");
    expect(readDraft().filters?.matching?.companies).toEqual(["Dollar General"]);
    expect(readDraft().sources).toHaveLength(2);
  });
  it("reveals bounded company results in pages beyond the first 50", async () => {
    vi.spyOn(api, "searchCompanies").mockResolvedValue(
      Array.from({ length: 75 }, (_, index) => ({
        name: `LocalCo ${String(index).padStart(3, "0")}`,
        catalog_company_id: null,
        jobs_seen: 1,
      })),
    );
    const user = userEvent.setup();
    render(<Editor />);
    await user.type(screen.getByRole("combobox", { name: "Companies (Optional)" }), "LocalCo");
    await screen.findByRole("option", { name: /LocalCo 000/ });
    expect(screen.getAllByRole("option")).toHaveLength(51);
    await user.click(screen.getByRole("button", { name: "Show more results" }));
    await screen.findByRole("option", { name: /LocalCo 074/ });
    expect(screen.getAllByRole("option")).toHaveLength(76);
  });
});
