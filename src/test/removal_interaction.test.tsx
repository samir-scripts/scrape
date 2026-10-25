// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { DeleteJobModal } from "../components/ui/DeleteJobModal";
import { RemovedJobsSettings } from "../components/views/settings/RemovedJobsSettings";
import { api, __resetDevData, __seedDevJobs, generateScrapedJobsForSource, SAMPLE_ENGINEERING_SOURCE } from "../lib/api";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); __resetDevData(); });
const job = generateScrapedJobsForSource(SAMPLE_ENGINEERING_SOURCE)[0];

describe("reversible removal controls", () => {
  it("starts with blocking unchecked, keeps cancellation independent, and restores focus", async () => {
    const confirm = vi.fn();
    function Harness() {
      const [open, setOpen] = useState(false);
      return <><button onClick={() => setOpen(true)}>Open removal</button><DeleteJobModal isOpen={open} job={job} copies={2} onClose={() => setOpen(false)} onConfirm={confirm} /></>;
    }
    render(<Harness />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Open removal" }));
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText(/All 2 saved copies/)).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
    await user.keyboard("{Enter}");
    expect(confirm).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Open removal" }));
    await user.click(screen.getByRole("button", { name: "Open removal" }));
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "Remove job" }));
    expect(confirm).toHaveBeenCalledExactlyOnceWith(true);
    fireEvent(screen.getByRole("dialog"), new Event("cancel", { bubbles: true }));
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Open removal" }));
  });
  it("unblocks without restoration, then restores saved notes through Settings", async () => {
    __seedDevJobs([{ ...job, notes: "Keep referral", favorited: true, status: "applied" }]);
    await api.removeJob(job.id, true);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={client}><RemovedJobsSettings /></QueryClientProvider>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Unblock future discovery" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "Unblock future discovery" })).toBeNull());
    expect((await api.listJobs()).items).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "Restore saved job and unblock" }));
    await waitFor(() => expect(screen.getByText(/No removed or blocked jobs/)).toBeTruthy());
    const restored = (await api.listJobs()).items[0];
    expect(restored.notes).toBe("Keep referral");
    expect(restored.favorited).toBe(true);
    expect(restored.status).toBe("applied");
  });
});
