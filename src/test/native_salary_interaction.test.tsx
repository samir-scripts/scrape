// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SalaryRangeFilter, type SalaryRangeFilterProps } from "../components/filters/SalaryRangeFilter";
import { matchesJob } from "../lib/filter";
import { generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";

beforeAll(() => {
  const values = new Map<string, string>();
  Object.defineProperty(window, "localStorage", { configurable: true, value: {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
  } });
});
afterEach(cleanup);
const initial = { salaryMin: null, salaryMax: null, currency: "USD", interval: "hour" as const, includeUnknown: false };
function Harness() {
  const [state, setState] = useState<Parameters<SalaryRangeFilterProps["onChange"]>[0]>(() => {
    const saved = window.localStorage.getItem("test-native-pay"); return saved ? JSON.parse(saved) : initial;
  });
  const fixture = generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0];
  return <><SalaryRangeFilter {...state} onChange={(values) => { setState(values); window.localStorage.setItem("test-native-pay", JSON.stringify(values)); }} />
    <output aria-label="Native filter">{JSON.stringify(state)}</output>
    <output aria-label="Fixture matches">{String(matchesJob(fixture, { salary_min: state.salaryMin, salary_max: state.salaryMax, salary_currency: state.currency, salary_interval: state.interval }))}</output></>;
}
describe("native hourly filter controls", () => {
  it("retains cents, rejects inverted input, restores persisted units and clears on mode/reset", async () => {
    window.localStorage.removeItem("test-native-pay");
    const mounted = render(<Harness />); const user = userEvent.setup();
    await user.type(screen.getByLabelText("Minimum per hour"), "50.25");
    await user.tab();
    await user.type(screen.getByLabelText("Maximum per hour"), "60.75");
    await user.tab();
    expect(screen.getByLabelText("Native filter").textContent).toContain('"salaryMin":50.25');
    expect(screen.getByLabelText("Fixture matches").textContent).toBe("true");
    await user.clear(screen.getByLabelText("Minimum per hour"));
    await user.type(screen.getByLabelText("Minimum per hour"), "70"); await user.tab();
    expect(screen.getByRole("alert").textContent).toContain("must not exceed");
    expect(screen.getByLabelText("Native filter").textContent).toContain('"salaryMin":50.25');
    mounted.unmount(); render(<Harness />);
    expect((screen.getByLabelText("Minimum per hour") as HTMLInputElement).value).toBe("50.25");
    await user.click(screen.getByRole("button", { name: "Annual" }));
    expect((screen.getByLabelText("Minimum per year") as HTMLInputElement).value).toBe("");
    expect(screen.getByLabelText("Fixture matches").textContent).toBe("false");
    await user.click(screen.getByRole("button", { name: "Clear compensation filter" }));
    expect(screen.getByLabelText("Native filter").textContent).toContain('"interval":null');
    expect(screen.getByLabelText("Fixture matches").textContent).toBe("true");
    window.localStorage.removeItem("test-native-pay");
  });
});
