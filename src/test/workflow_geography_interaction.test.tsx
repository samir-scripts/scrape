// @vitest-environment jsdom
import { importWorkflowYaml } from "../lib/workflowImport";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { GeographyMatching } from "../components/views/settings/GeographyMatching";
import { emptyRules } from "../components/views/settings/WorkflowSections";
import type { PostingRules } from "../bindings/PostingRules";
import { sourceToWorkflowSpec, workflowSpecToYaml, workflowSpecToCreateInput } from "../lib/yamlWorkflow";
import { SAMPLE_ENGINEERING_SOURCE } from "../lib/api";

afterEach(cleanup);
function Harness({ initial, invalidate }: { initial?: Partial<PostingRules>; invalidate: () => void }) {
  const [rules,setRules] = useState({ ...emptyRules(), ...initial });
  return <><GeographyMatching rules={rules} onChange={patch => setRules(current => ({...current,...patch}))} onDraftChange={invalidate} /><output data-testid="rules">{JSON.stringify(rules)}</output></>;
}
const rules = () => JSON.parse(screen.getByTestId("rules").textContent!);
describe("workflow geography and workplace", () => {
  it("searches country and state, combines workplaces, clears incompatible states and validates drafts", async () => {
    const user=userEvent.setup(), invalidate=vi.fn();
    render(<Harness invalidate={invalidate} />);
    const country=screen.getByRole("combobox",{name:"Country"}), state=screen.getByRole("combobox",{name:"State/province"});
    expect((state as HTMLInputElement).disabled).toBe(true);
    await user.type(country,"US"); await user.tab();
    expect(rules().countries).toEqual(["US"]);
    await user.type(state,"CA"); await user.keyboard("{Enter}");
    expect(rules().regions).toEqual(["US-CA"]);
    await user.click(screen.getByRole("checkbox",{name:"Remote"}));
    await user.click(screen.getByRole("checkbox",{name:"Hybrid"}));
    expect(rules().workplaces).toEqual(["remote","hybrid"]);
    await user.click(screen.getByRole("checkbox",{name:"Include postings with unknown location"}));
    expect(rules().location_include_unknown).toBe(true);
    await user.clear(country); await user.type(country,"Canada"); await user.tab();
    expect(rules().countries).toEqual(["CA"]); expect(rules().regions).toEqual([]);
    await user.type(state,"Ontario"); await user.tab();
    expect(rules().regions).toEqual(["CA-ON"]);
    await user.click(screen.getByRole("button",{name:"Remove Ontario"}));
    expect(rules().regions).toEqual([]);
    await user.clear(country); await user.type(country,"Unlisted country");
    expect(country.getAttribute("aria-invalid")).toBe("true");
    expect(rules().countries).toEqual(["CA"]);
    await user.keyboard("{Escape}");
    expect((country as HTMLInputElement).value).toBe("Canada");
    expect(invalidate).toHaveBeenCalled();
  });
  it("retains imported multiple countries and subdivisions until an explicit country change", async () => {
    const user=userEvent.setup();
    render(<Harness invalidate={vi.fn()} initial={{countries:["US","CA"],regions:["US-CA","CA-ON"]}} />);
    const country=screen.getByRole("combobox",{name:"Country"});
    expect(country.getAttribute("aria-invalid")).toBe("false");
    await user.click(screen.getByRole("checkbox",{name:"In-person / On-site"}));
    expect(rules().countries).toEqual(["US","CA"]);
    expect(rules().regions).toEqual(["US-CA","CA-ON"]);
    await user.clear(country); await user.type(country,"United Kingdom"); await user.tab();
    expect(rules().countries).toEqual(["GB"]); expect(rules().regions).toEqual([]);
  });
  it("suggests multiword country and state names plus known city terms without rejecting custom cities", async () => {
    const user = userEvent.setup();
    render(<Harness invalidate={vi.fn()} />);
    const country = screen.getByRole("combobox", { name: "Country" });
    await user.click(country);
    await user.paste("United States");
    await user.click(await screen.findByRole("option", { name: /United States \(US\)/ }));
    expect(rules().countries).toEqual(["US"]);

    const state = screen.getByRole("combobox", { name: "State/province" });
    await user.type(state, "New Jersey");
    await user.click(await screen.findByRole("option", { name: /New Jersey \(US-NJ\)/ }));
    expect(rules().regions).toEqual(["US-NJ"]);

    const cities = screen.getByRole("combobox", { name: "Cities / Location terms" });
    await user.type(cities, "Seat");
    expect(await screen.findByRole("option", { name: "Seattle" })).toBeTruthy();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(rules().cities).toEqual(["Seattle"]);
    await user.type(cities, "Springfield{Enter}");
    expect(rules().cities).toEqual(["Seattle", "Springfield"]);
  });
  it("round trips canonical geography through YAML and source editing", async () => {
    const spec=sourceToWorkflowSpec(SAMPLE_ENGINEERING_SOURCE);
    spec.filters={matching:{...emptyRules(),countries:["US"],regions:["US-CA"],workplaces:["remote","hybrid"],location_include_unknown:true}};
    const yaml=workflowSpecToYaml(spec);
    expect(yaml).toContain("US-CA");
    const validated=await importWorkflowYaml(yaml);
    const input=workflowSpecToCreateInput(validated);
    const reopened=sourceToWorkflowSpec({...SAMPLE_ENGINEERING_SOURCE,config:input.config ?? null,filters:input.filters ?? null});
    expect(reopened.filters?.matching).toEqual(spec.filters.matching);
    expect(validated.filters?.matching).toEqual(spec.filters.matching);
  });
});
