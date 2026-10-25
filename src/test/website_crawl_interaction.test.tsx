// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowSections } from "../components/views/settings/WorkflowSections";
import { api, SAMPLE_ENGINEERING_SOURCE } from "../lib/api";
import { sourceToWorkflowSpec, workflowSpecToYaml, validateWorkflowYaml } from "../lib/yamlWorkflow";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it.each(["light", "dark"])("%s crawl controls update one draft and preserve YAML/reopen configuration", async theme => {
  vi.spyOn(api, "getAdzunaCredentialStatus").mockResolvedValue({ configured: false });
  const initial = sourceToWorkflowSpec(SAMPLE_ENGINEERING_SOURCE);
  initial.sources = [{ ...initial.sources[0], extractor: { kind: "selector", url: "https://careers.example/jobs", source_config: {}, mode: "static" } }];
  let saved = initial;
  function Harness({ spec }: { spec: WorkflowSpec }) {
    const [draft, change] = useState(spec);
    saved = draft;
    return <div data-theme={theme}><WorkflowSections draft={draft} change={change} advanced section="Sources" setTargetCompanies={() => {}} attachCompanyBoard={() => {}} /></div>;
  }
  const user = userEvent.setup();
  const mounted = render(<Harness spec={initial} />);
  await user.click(screen.getByRole("checkbox", { name: "Source 1 website crawl" }));
  expect(screen.getByLabelText("Source 1 crawl page cap")).toHaveProperty("value", "20");
  expect(screen.getByLabelText("Source 1 crawl depth")).toHaveProperty("value", "2");
  await user.clear(screen.getByLabelText("Source 1 crawl page cap"));
  await user.type(screen.getByLabelText("Source 1 crawl page cap"), "7");
  await user.click(screen.getByRole("button", { name: "Source 1 crawl extraction" }));
  await user.click(screen.getByRole("option", { name: "Configured selectors" }));
  expect(saved.sources[0].extractor.source_config).toMatchObject({ crawl: { max_pages: 7, max_depth: 2, extraction: "selectors" } });
  // JSON-LD requires no configured item/title/link selectors.
  await user.click(screen.getByRole("button", { name: "Source 1 crawl extraction" }));
  await user.click(screen.getByRole("option", { name: "JobPosting JSON-LD" }));
  const yaml = workflowSpecToYaml(saved);
  const reopened = validateWorkflowYaml(yaml);
  expect(reopened.parsedSpec?.sources[0].extractor.source_config).toEqual(saved.sources[0].extractor.source_config);
  mounted.unmount();
  render(<Harness spec={reopened.parsedSpec!} />);
  expect(screen.getByLabelText("Source 1 crawl page cap")).toHaveProperty("value", "7");
  expect(screen.getByRole("checkbox", { name: "Source 1 website crawl" })).toHaveProperty("checked", true);
  expect(screen.getByText(/not internet-wide search/)).toBeTruthy();
  await user.click(screen.getByRole("checkbox", { name: "Source 1 website crawl" }));
  expect(saved.sources[0].extractor.source_config).toEqual({});
});
