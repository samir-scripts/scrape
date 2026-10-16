// @vitest-environment jsdom
import { api } from "../lib/api";
import { clearCompanyLogoCache } from "../lib/companyLogoCache";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { CompanyLogo } from "../components/ui/CompanyLogo";

afterEach(() => {cleanup();vi.restoreAllMocks();clearCompanyLogoCache();});

it("falls back for a failed image and attempts newly supplied branding", () => {
  const { rerender } = render(<CompanyLogo company="Fixture Company" iconUrl="https://fixture.test/old.png" />);
  fireEvent.error(screen.getByRole("img", { name: "Fixture Company" }));
  expect(screen.queryByRole("img")).toBeNull();
  expect(screen.getByLabelText("Fixture Company initials").textContent).toBe("FC");

  rerender(<CompanyLogo company="Fixture Company" iconUrl="https://fixture.test/new.png" />);
  expect(screen.getByRole("img", { name: "Fixture Company" }).getAttribute("src")).toBe("https://fixture.test/new.png");
});

it("uses cached artwork offline, retries after refresh and distinguishes missing metadata from display failure", () => {
  const cached={ evidence:{url:"https://offline.test/logo.png",page_url:"https://offline.test/jobs",method:"employer_label" as const}, checked_at:"2026-10-16T00:00:00Z",status:"verified" as const,asset:"data:image/png;base64,cached-fixture",diagnostic:"Using last verified logo. Latest lookup failed." };
  const {rerender}=render(<CompanyLogo company="Fixture Company" iconUrl="https://offline.test/logo.png" logo={cached} />);
  expect(screen.getByRole("img").getAttribute("src")).toBe(cached.asset);
  fireEvent.error(screen.getByRole("img"));
  expect(screen.getByLabelText("Fixture Company initials").getAttribute("data-logo-status")).toBe("display_failed");
  rerender(<CompanyLogo company="Fixture Company" logo={{...cached,checked_at:"2026-10-17T00:00:00Z"}} />);
  expect(screen.getByRole("img").getAttribute("src")).toBe(cached.asset);
  rerender(<CompanyLogo company="Fixture Company" logo={{...cached,asset:null,status:"missing_metadata",diagnostic:null}} />);
  expect(screen.getByLabelText("Fixture Company initials").title).toContain("No employer logo metadata");
});

it("shares a local asset read across cards and can clear the cache after reset", async () => {
 const reference="sha256:"+"a".repeat(64),asset="data:image/png;base64,local-fixture";
 const read=vi.spyOn(api,"getCompanyLogoAsset").mockResolvedValue(asset);
 const logo={evidence:null,checked_at:"2026-10-16T00:00:00Z",status:"verified" as const,asset:null,cached_asset_reference:reference,diagnostic:null};
 const {unmount}=render(<><CompanyLogo company="One" logo={logo}/><CompanyLogo company="Two" logo={logo}/></>);
 await waitFor(()=>expect(screen.getAllByRole("img")).toHaveLength(2));expect(read).toHaveBeenCalledTimes(1);
 expect(screen.getByRole("img",{name:"One"}).getAttribute("src")).toBe(asset);
 unmount();clearCompanyLogoCache();render(<CompanyLogo company="After reset" logo={logo}/>);
 await screen.findByRole("img",{name:"After reset"});expect(read).toHaveBeenCalledTimes(2);
});
