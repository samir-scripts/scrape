// @vitest-environment jsdom
vi.mock("../lib/runtime", async importOriginal => ({
  ...await importOriginal<typeof import("../lib/runtime")>(),
  isDemoMode: () => false,
}));
import { api } from "../lib/api";
import { clearCompanyLogoCache } from "../lib/companyLogoCache";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { CompanyLogo } from "../components/ui/CompanyLogo";

afterEach(() => {cleanup();vi.restoreAllMocks();clearCompanyLogoCache();});

it("never displays an unverified icon URL and uses initials when verified artwork is absent", () => {
  const html = render(<CompanyLogo company="Fixture Company" />);
  expect(screen.queryByRole("img")).toBeNull();
  expect(screen.getByLabelText("Fixture Company initials").textContent).toBe("FC");
  html.unmount();
});

it("uses cached artwork offline, retries after refresh and distinguishes missing metadata from display failure", () => {
  const cached={ evidence:{url:"https://offline.test/logo.png",page_url:"https://offline.test/jobs",method:"employer_label" as const}, checked_at:"2026-10-16T00:00:00Z",status:"verified" as const,asset:"data:image/png;base64,cached-fixture",diagnostic:"Using last verified logo. Latest lookup failed." };
  const {rerender}=render(<CompanyLogo company="Fixture Company" logo={cached} />);
  expect(screen.getByRole("img").getAttribute("src")).toBe(cached.asset);
  fireEvent.error(screen.getByRole("img"));
  expect(screen.getByLabelText("Fixture Company initials").getAttribute("data-logo-status")).toBe("display_failed");
  rerender(<CompanyLogo company="Fixture Company" logo={{...cached,checked_at:"2026-10-17T00:00:00Z"}} />);
  expect(screen.getByRole("img").getAttribute("src")).toBe(cached.asset);
  rerender(<CompanyLogo company="Fixture Company" logo={{...cached,asset:null,status:"missing_metadata",diagnostic:null}} />);
  expect(screen.getByLabelText("Fixture Company initials").title).toContain("No employer logo metadata");
});

it("automatically resolves visible persisted jobs and coalesces same source/employer lookups", async () => {
  const asset="data:image/png;base64,verified-fixture";
  const report={evidence:{url:"https://employer.test/logo.png",page_url:"https://jobs.ashbyhq.com/employer/1",method:"provider" as const},checked_at:"2026-10-20T00:00:00Z",status:"verified" as const,asset,cached_asset_reference:null,diagnostic:null};
  const lookup=vi.spyOn(api,"ensureCompanyLogoForJob").mockResolvedValue(report);
  render(<><CompanyLogo company="Acme" jobId="job-1" sourceId="source" /><CompanyLogo company=" ACME " jobId="job-2" sourceId="source" /></>);
  await waitFor(()=>expect(screen.getAllByRole("img")).toHaveLength(2));
  expect(lookup).toHaveBeenCalledTimes(1);
  expect(lookup).toHaveBeenCalledWith("job-1");
  expect(screen.getAllByRole("img").every(image=>image.getAttribute("src")===asset)).toBe(true);
});

it("retains initials and negative-caches a failed lookup across row remounts", async () => {
  const lookup=vi.spyOn(api,"ensureCompanyLogoForJob").mockResolvedValue(null);
  const props={company:"Acme",jobId:"job-1",sourceId:"source"};
  const {unmount}=render(<CompanyLogo {...props}/>);
  await waitFor(()=>expect(lookup).toHaveBeenCalledTimes(1));
  expect(screen.getByLabelText("Acme initials")).toBeTruthy();
  unmount();
  render(<CompanyLogo {...props} jobId="job-2"/>);
  await waitFor(()=>expect(screen.getByLabelText("Acme initials")).toBeTruthy());
  expect(lookup).toHaveBeenCalledTimes(1);
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
