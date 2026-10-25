// @vitest-environment jsdom
import { useRef } from "react";
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowSections } from "../components/views/settings/WorkflowSections";
import { SAMPLE_WORKFLOW_TEMPLATES, workflowSpecToYaml, validateWorkflowYaml } from "../lib/yamlWorkflow";
import { useWorkflowDraft } from "../hooks/useWorkflowDraft";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";

afterEach(cleanup);
function Harness({ companies = [] }: { companies?: string[] }) {
  const initial = useRef(structuredClone(SAMPLE_WORKFLOW_TEMPLATES[0].spec));
  initial.current.filters!.matching!.companies = companies;
  const model = useWorkflowDraft({ isOpen: true, initialSpec: initial.current, onSourceSaved: () => {} });
  return <>
    <WorkflowSections draft={model.draft} change={model.change} advanced={false} section="Roles" setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} />
    <WorkflowSections draft={model.draft} change={model.change} advanced={false} section="Criteria" setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} />
    <WorkflowSections draft={model.draft} change={model.change} advanced={false} section="Sources" setTargetCompanies={model.setTargetCompanies} attachCompanyBoard={model.attachCompanyBoard} />
    <output data-testid="draft">{workflowSpecToYaml(model.draft)}</output>
  </>;
}
it("explicitly combines known and unknown evidence choices without mutating the catalog", async () => {
  const user = userEvent.setup();
  render(<Harness />);
  const template = SAMPLE_WORKFLOW_TEMPLATES[0];
  await user.click(screen.getByRole("checkbox", { name: "Full-time" }));
  await user.click(screen.getByRole("checkbox", { name: "Unknown employment" }));
  await user.click(screen.getByRole("checkbox", { name: "Remote" }));
  await user.click(screen.getByRole("checkbox", { name: "Unknown workplace" }));
  const spec = validateWorkflowYaml(screen.getByTestId("draft").textContent!).parsedSpec!;
  expect(spec.filters?.matching?.role_types).toEqual(["full_time", "unknown"]);
  expect(spec.filters?.matching?.workplaces).toEqual(["remote", "unknown"]);
  expect(spec.filters?.matching?.location_include_unknown).toBe(false);
  // A new draft must never mutate the canonical data shared by other editors.
  expect(template.spec.filters?.matching?.role_types).toEqual([]);
});
it("keeps example loading in Advanced and requires whole-draft replacement confirmation", async () => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
  const user = userEvent.setup();
  render(<AddSourceModal isOpen onClose={() => {}} onSourceSaved={() => {}} />);
  await user.clear(screen.getByRole("textbox", { name: "Workflow name" }));
  await user.type(screen.getByRole("textbox", { name: "Workflow name" }), "Keep my edits");
  expect(screen.queryByRole("button", { name: "Load example workflow" })).toBeNull();
  await user.click(screen.getByRole("button", { name: "Advanced" }));
  await user.click(screen.getByRole("button", { name: "Load example workflow" }));
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "remote-usd-over-60000")!;
  await user.click(screen.getByRole("option", { name: template.name }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe("Keep my edits");
  await user.click(screen.getByRole("button", { name: "Keep editing" }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe("Keep my edits");
  await user.click(screen.getByRole("button", { name: "Load example workflow" }));
  await user.click(screen.getByRole("option", { name: template.name }));
  await user.click(screen.getByRole("button", { name: "Replace draft" }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe(template.name);
  await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
  await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
  expect((screen.getByRole("checkbox", { name: "Enable Adzuna not enabled — setup required" }) as HTMLInputElement).checked).toBe(false);
  expect(screen.queryByRole("button", { name: "Save disabled for review" })).toBeNull();
  expect(screen.queryByRole("checkbox", { name: /^Enable (Netflix|Amazon)$/ })).toBeNull();
  expect(screen.getByRole("checkbox", { name: "Enable Jobicy — remote; seven-day feed, three-hour delay" })).toBeTruthy();
});

it("keeps independent gated company choices available without changing canonical matching or enabling them", async () => {
  const user = userEvent.setup();
  render(<Harness companies={["Google", "Netflix"]} />);
  await user.click(screen.getByRole("button", { name: "Attach career board for Google" }));
  await user.click(screen.getByRole("button", { name: "Attach career board for Netflix" }));
  const spec = validateWorkflowYaml(screen.getByTestId("draft").textContent!).parsedSpec!;
  const google = spec.sources.find(source => source.employer_hint === "Google")!;
  const netflix = spec.sources.find(source => source.employer_hint === "Netflix")!;
  expect(google.enabled).toBe(false);
  expect(google.extractor.mode).toBe("static");
  expect(netflix.enabled).toBe(false);
  expect(netflix.extractor.mode).toBe("script");
  expect(netflix.extractor.script?.code).toContain("version: 2");
  expect(netflix.extractor.script?.interpreter).toBeUndefined();
  expect(spec.filters?.matching?.target_roles).toEqual(SAMPLE_WORKFLOW_TEMPLATES[0].spec.filters?.matching?.target_roles);
  expect(spec.filters?.matching?.posting_date?.preset).toBe("past_12h");
  expect(SAMPLE_WORKFLOW_TEMPLATES.map(template => template.id)).toEqual([
    "software-engineering-last-12-hours", "cherry-hill-fast-food-last-month", "remote-usd-over-60000",
  ]);
});
