// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
import { api, DEFAULT_SETTINGS, SAMPLE_SWE_SOURCE } from "../lib/api";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import { runRequiresAttention } from "../lib/status";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
it("allows saving a preview with optional warnings and blocks incomplete required evidence", async () => {
  const user=userEvent.setup();
  let required=false;
  vi.spyOn(api,"previewWorkflow").mockImplementation(async spec => ({spec,branding:[],jobs:[],fetched:1,matched:0,rejected:0,invalid:0,blocked:0,sampled:false,next_run_at:null,errors:required?["Required publication date unavailable; retry the source."]:[],warnings:required?[]:["Optional publication date enrichment unavailable."]} as WorkflowPreview));
  const save=vi.spyOn(api,"saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
  render(<AddSourceModal isOpen initialSource={SAMPLE_SWE_SOURCE} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
  await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
  await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
  await user.click(screen.getByRole("button", { name: "Continue to Review" }));
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  expect(await screen.findByText(/Warning: Optional publication date/)).toBeTruthy();
  await waitFor(() => expect((screen.getByRole("button",{name:"Save workflow"}) as HTMLButtonElement).disabled).toBe(false));
  required=true;
  await user.click(screen.getByRole("button",{name:"Refresh preview"}));
  await screen.findByRole("alert");
  expect((screen.getByRole("button",{name:"Save workflow"}) as HTMLButtonElement).disabled).toBe(true);
  expect(save).not.toHaveBeenCalled();
});
it("shows warning, partial and cancelled workflow outcomes without counting cancellation as an error", async () => {
  const sources=[
    {...SAMPLE_SWE_SOURCE,id:"warning",name:"Warning fixture",last_status:"success",last_outcome:"success_with_warnings" as const},
    {...SAMPLE_SWE_SOURCE,id:"partial",name:"Partial fixture",last_status:"error",last_outcome:"partial" as const},
    {...SAMPLE_SWE_SOURCE,id:"cancel",name:"Cancelled fixture",last_status:"error",last_outcome:"cancelled" as const},
  ];
  vi.spyOn(api,"listSources").mockResolvedValue(sources);
  render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
  await screen.findByRole("button",{name:"Edit workflow Warning fixture"});
  expect(screen.getByText("SUCCESS WITH WARNINGS")).toBeTruthy();
  expect(screen.getByText("CANCELLED")).toBeTruthy();
  const user=userEvent.setup();await user.click(screen.getByRole("button",{name:"ERRORS (1)"}));
  expect(screen.getByRole("button",{name:"Edit workflow Partial fixture"})).toBeTruthy();
  expect(screen.queryByRole("button",{name:"Edit workflow Cancelled fixture"})).toBeNull();
  expect(screen.queryByRole("button",{name:"Edit workflow Warning fixture"})).toBeNull();
  expect(runRequiresAttention({status:"success_with_warnings",error:"Optional warning"})).toBe(false);
  expect(runRequiresAttention({status:"cancelled",error:"Explicit cancellation"})).toBe(false);
});
