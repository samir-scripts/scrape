// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { JobCard } from "../components/views/discovered/JobCard";
import { generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";

afterEach(cleanup);

describe("independent card keyboard controls", () => {
  it("opens the stored destination without activating details; delete/favorite also stay independent", async () => {
    const job = generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0];
    const details = vi.fn(), open = vi.fn(), remove = vi.fn(), favorite = vi.fn();
    render(<JobCard job={job} onClick={details} onOpenUrl={open} onDelete={remove} onToggleFavorite={favorite} />);
    const user = userEvent.setup();
    screen.getByRole("button", { name: /^Open .* in browser$/ }).focus();
    await user.keyboard("{Enter}");
    expect(open).toHaveBeenCalledExactlyOnceWith(job.url);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: /^Delete / }).focus();
    await user.keyboard(" ");
    expect(remove).toHaveBeenCalledExactlyOnceWith(job);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: "Favorite posting" }).focus();
    await user.keyboard("{Enter}");
    expect(favorite).toHaveBeenCalledExactlyOnceWith(job.id);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: /^View / }).focus();
    await user.keyboard("{Enter}");
    expect(details).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: /^Delete / }).querySelector("svg")).not.toBeNull();
  });
});
