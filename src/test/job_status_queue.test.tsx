// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { Job } from "../bindings/Job";
import type { ReactNode } from "react";
import { useJobStatusChange } from "../hooks/useJobStatusChange";
import { recordedJobs } from "../lib/recordedJobs";
import { api } from "../lib/api";
import { useToastStore } from "../lib/toast";
const job = { ...recordedJobs()[0], id: "queue-job", status: "applied" as const };
afterEach(() => { cleanup(); vi.restoreAllMocks(); useToastStore.getState().clearAll(); });
function mount() {
 const changed = vi.fn(), client = new QueryClient();
 const result = renderHook(({ jobs }: { jobs: Job[] }) => useJobStatusChange(jobs, changed), { initialProps: { jobs: [job] as Job[] }, wrapper: ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider> });
 return { ...result, changed };
}
it("serializes rapid per-job changes, ignores no-ops and records the confirmed previous stage", async () => {
 let complete!: (value: Job) => void;
 const update = vi.spyOn(api, "updateJobStatus").mockImplementationOnce(() => new Promise(resolve => { complete = resolve; })).mockImplementation(async (_, status) => ({ ...job, status }));
 const { result, changed } = mount();
 let first!: ReturnType<typeof result.current.changeStatus>, second!: typeof first, duplicate!: typeof first;
 act(() => { first = result.current.changeStatus(job.id, "interviewing"); second = result.current.changeStatus(job.id, "offer"); duplicate = result.current.changeStatus(job.id, "offer"); });
 await act(async () => { await Promise.resolve(); await Promise.resolve(); });
 expect(update).toHaveBeenCalledTimes(1); expect(result.current.pendingIds.has(job.id)).toBe(true);
 await act(async () => { complete({ ...job, status: "interviewing" }); await Promise.all([first, second, duplicate]); });
 expect(update.mock.calls.map(call => call[1])).toEqual(["interviewing", "offer"]);
 expect(changed.mock.calls.map(call => [call[0].status, call[1].status])).toEqual([["applied", "interviewing"], ["interviewing", "offer"]]);
 expect(result.current.pendingIds.size).toBe(0);
});
it("keeps confirmed state on failure, permits a newer queued success, and retries against that newer state", async () => {
 const update = vi.spyOn(api, "updateJobStatus").mockRejectedValueOnce(new Error("disk full")).mockImplementation(async (_, status) => ({ ...job, status }));
 const { result, changed, rerender } = mount();
 await act(async () => { const failed = result.current.changeStatus(job.id, "interviewing"); const next = result.current.changeStatus(job.id, "offer"); await Promise.all([failed, next]); });
 expect(changed.mock.calls.map(call => [call[0].status, call[1].status])).toEqual([["applied", "offer"]]);
 rerender({ jobs: [{ ...job, status: "offer" }] });
 const toast = useToastStore.getState().toasts[0]; expect(toast.message).toContain("saved stage was kept");
 await act(async () => { toast.action!.run(); await new Promise(resolve => setTimeout(resolve, 0)); });
 expect(update).toHaveBeenLastCalledWith(job.id, "interviewing");
 expect(changed.mock.calls.at(-1)?.[0].status).toBe("offer"); expect(result.current.pendingIds.size).toBe(0);
});

it("uses refreshed repository state after a queue drains", async () => {
 const update = vi.spyOn(api, "updateJobStatus").mockImplementation(async (_, status) => ({ ...job, status }));
 const client = new QueryClient();
 const { result, rerender } = renderHook(({ jobs }: { jobs: Job[] }) => useJobStatusChange(jobs, vi.fn()), { initialProps: { jobs: [job] as Job[] }, wrapper: ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider> });
 await act(async () => { await result.current.changeStatus(job.id, "interviewing"); });
 rerender({ jobs: [{ ...job, status: "offer" }] });
 await act(async () => { await result.current.changeStatus(job.id, "interviewing"); });
 expect(update).toHaveBeenCalledTimes(2);
});
