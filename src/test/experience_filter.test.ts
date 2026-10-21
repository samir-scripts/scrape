// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createElement, useState } from "react";
import { ExperienceFilter } from "../components/filters/ExperienceFilter";
import type { ExperienceFilterProps } from "../components/filters/ExperienceFilter";

type Values = Omit<ExperienceFilterProps, "onChange" | "onValidityChange" | "onDraftChange">;
const empty: Values = {
  experienceMin: null, experienceMax: null, experienceMatchMode: "required_minimum",
  newGrad: "any", includeUnknown: false,
};
afterEach(cleanup);

function Harness({ initial = empty }: { initial?: Values }) {
  const [values, setValues] = useState(initial);
  const [valid, setValid] = useState(true);
  return createElement("div", null,
    createElement(ExperienceFilter, { ...values, onChange: setValues, onValidityChange: setValid }),
    createElement("output", { "data-testid": "values" }, JSON.stringify(values)),
    createElement("button", { disabled: !valid }, "Save"));
}

function currentValues(): Values {
  return JSON.parse(screen.getByTestId("values").textContent || "{}");
}

describe("experience matching controls", () => {
  it("keeps numeric required experience independent from New Grad eligibility", () => {
    render(createElement(Harness));
    fireEvent.click(screen.getByRole("button", { name: "0–2" }));
    expect(currentValues()).toMatchObject({ experienceMin: 0, experienceMax: 2, newGrad: "any", experienceMatchMode: "required_minimum", includeUnknown: false });
    fireEvent.click(screen.getByRole("button", { name: "New Grad only" }));
    fireEvent.click(screen.getByRole("button", { name: "5+" }));
    expect(currentValues()).toMatchObject({ experienceMin: 5, experienceMax: null, newGrad: "only" });
    fireEvent.click(screen.getByRole("button", { name: "Any" }));
    expect(currentValues()).toMatchObject({ experienceMin: null, experienceMax: null, newGrad: "only" });
  });

  it("preserves saved overlap and unknown settings on bounds edits and converts only explicitly", () => {
    const initial: Values = { ...empty, experienceMin: 1, experienceMax: 5, experienceMatchMode: "advertised_range_overlap", includeUnknown: true };
    render(createElement(Harness, { initial }));
    expect(screen.getByText("Advertised range overlap (saved rule)")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Minimum experience (years)"), { target: { value: "3" } });
    expect(currentValues()).toMatchObject({ experienceMin: 3, experienceMatchMode: "advertised_range_overlap", includeUnknown: true });
    fireEvent.click(screen.getByRole("button", { name: "0–1" }));
    expect(currentValues()).toMatchObject({ experienceMatchMode: "advertised_range_overlap", includeUnknown: true });
    fireEvent.click(screen.getByRole("button", { name: "Use minimum required experience" }));
    const saved = currentValues();
    cleanup();
    render(createElement(Harness, { initial: saved }));
    expect(screen.getByText("Minimum required experience")).toBeTruthy();
    expect(currentValues()).toMatchObject({ experienceMin: 0, experienceMax: 1, experienceMatchMode: "required_minimum", includeUnknown: true });
  });

  it("clears the numeric group and starts new constraints without unknown inclusion", () => {
    render(createElement(Harness, { initial: { ...empty, experienceMin: 1, includeUnknown: true, experienceMatchMode: "advertised_range_overlap" } }));
    fireEvent.click(screen.getByRole("button", { name: "Any" }));
    fireEvent.click(screen.getByRole("button", { name: "3–5" }));
    expect(currentValues()).toMatchObject({ experienceMin: 3, experienceMax: 5, experienceMatchMode: "required_minimum", includeUnknown: false });
    fireEvent.click(screen.getByRole("checkbox", { name: "Include jobs with unknown required experience" }));
    fireEvent.change(screen.getByLabelText("Maximum experience (years)"), { target: { value: "6" } });
    expect(currentValues().includeUnknown).toBe(true);
  });

  it.each(["1.5", "-1", "2years", "1e2", "9007199254740992"])("rejects invalid whole-year input %s without truncating it", (value) => {
    const change = vi.fn();
    render(createElement(ExperienceFilter, { ...empty, onChange: change }));
    const min = screen.getByLabelText("Minimum experience (years)");
    fireEvent.change(min, { target: { value } });
    expect(min.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByRole("alert")).toBeTruthy();
    expect(change).not.toHaveBeenCalled();
  });

  it("gates saving for inverted inclusive bounds and recovers when corrected", () => {
    render(createElement(Harness, { initial: { ...empty, experienceMin: 0, experienceMax: 2 } }));
    fireEvent.change(screen.getByLabelText("Minimum experience (years)"), { target: { value: "3" } });
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
    expect(currentValues().experienceMin).toBe(0);
    fireEvent.change(screen.getByLabelText("Maximum experience (years)"), { target: { value: "5" } });
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(false);
    expect(currentValues()).toMatchObject({ experienceMin: 3, experienceMax: 5 });
  });
});

it("invalidates a current preview even when an invalid edit cannot update numeric rules", () => {
  const change = vi.fn();
  const invalidatePreview = vi.fn();
  const validity = vi.fn();
  render(createElement(ExperienceFilter, {
    ...empty, experienceMin: 0, experienceMax: 2,
    onChange: change, onDraftChange: invalidatePreview, onValidityChange: validity,
  }));
  fireEvent.change(screen.getByLabelText("Maximum experience (years)"), { target: { value: "2.5" } });
  expect(invalidatePreview).toHaveBeenCalledTimes(1);
  expect(validity).toHaveBeenLastCalledWith(false);
  expect(change).not.toHaveBeenCalled();
});
