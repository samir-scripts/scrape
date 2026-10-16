// @vitest-environment jsdom
import { afterEach, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { CompanyLogo } from "../components/ui/CompanyLogo";

afterEach(cleanup);

it("falls back for a failed image and attempts newly supplied branding", () => {
  const { rerender } = render(<CompanyLogo company="Fixture Company" iconUrl="https://fixture.test/old.png" />);
  fireEvent.error(screen.getByRole("img", { name: "Fixture Company" }));
  expect(screen.queryByRole("img")).toBeNull();
  expect(screen.getByLabelText("Fixture Company initials").textContent).toBe("FC");

  rerender(<CompanyLogo company="Fixture Company" iconUrl="https://fixture.test/new.png" />);
  expect(screen.getByRole("img", { name: "Fixture Company" }).getAttribute("src")).toBe("https://fixture.test/new.png");
});
