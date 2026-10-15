// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { isTauri } from "@tauri-apps/api/core";
import { ScheduleControl } from "../components/layout/ScheduleControl";
import { schedulePresentation, scheduleTime } from "../lib/schedulePresentation";
import { api } from "../lib/api";
import type { SchedulerStatus } from "../bindings/SchedulerStatus";
import { useUIStore } from "../lib/store";
vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn(() => true), invoke: vi.fn() }));
const scheduled: SchedulerStatus = { enabled: true, running: false, enabled_workflows: 2, next_run_at: "2027-01-01T02:00:00.000Z", schedules: [
 {id:"a",name:"Acme",timezone:"America/New_York",next_run_at:"2027-01-01T02:00:00.000Z"},
 {id:"b",name:"Beta",timezone:"UTC",next_run_at:"2027-01-01T02:00:00.000Z"},
] };
beforeAll(() => { HTMLDialogElement.prototype.showModal = function() { this.setAttribute("open", ""); }; HTMLDialogElement.prototype.close = function() { this.removeAttribute("open"); }; });
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.mocked(isTauri).mockReturnValue(true); useUIStore.getState().setScanning(false, ""); });
function mount() { const client = new QueryClient({defaultOptions:{queries:{retry:false,gcTime:0}}}); render(<QueryClientProvider client={client}><ScheduleControl /></QueryClientProvider>); return client; }
it("formats one canonical status with date rollover, ties, unknown instants and both running/paused facts", () => {
 expect(scheduleTime(scheduled.next_run_at, "America/New_York")).toMatch(/Dec 31, 2026/);
 expect(schedulePresentation(scheduled).detail).toContain("Acme, Beta");
 expect(schedulePresentation({...scheduled,enabled:false,running:true}).label).toBe("Running · Paused");
 expect(schedulePresentation({...scheduled,enabled_workflows:0,schedules:[],next_run_at:null}).label).toBe("No enabled workflows");
 expect(schedulePresentation({...scheduled,schedules:[],next_run_at:null}).label).toBe("Manual only");
 for (const value of [null,"not a timestamp"]) expect(schedulePresentation({...scheduled,next_run_at:value}).label).toBe("Schedule needs attention");
 expect(scheduleTime(scheduled.next_run_at,"bad/timezone")).toBeNull();
 expect(schedulePresentation(undefined,undefined,true).label).toBe("Loading schedule");
 expect(schedulePresentation(scheduled,new Error("IPC failed")).label).toBe("Schedule unavailable");
});
it("opens via keyboard, shows workflow timezones, pauses/resumes and restores focus on Escape", async () => {
 let status = scheduled;
 vi.spyOn(api,"getSchedulerStatus").mockImplementation(async () => status);
 const pause = vi.spyOn(api,"setSchedulerPaused").mockImplementation(async paused => {status={...scheduled,enabled:!paused,next_run_at:paused?null:scheduled.next_run_at};});
 mount(); const user=userEvent.setup(); const trigger=await screen.findByRole("button",{name:/Schedule: Next/}); trigger.focus(); await user.keyboard("{Enter}");
 await screen.findByRole("dialog",{name:"Schedule summary"}); expect(screen.getByText("America/New_York")).toBeTruthy();
 await user.click(screen.getByRole("button",{name:"Pause automatic runs"})); await screen.findByRole("button",{name:"Resume automatic runs"}); expect(pause).toHaveBeenCalledWith(true);
 await user.click(screen.getByRole("button",{name:"Resume automatic runs"})); await screen.findByRole("button",{name:"Pause automatic runs"}); expect(pause).toHaveBeenLastCalledWith(false);
 await user.keyboard("{Escape}"); await waitFor(()=>expect(document.activeElement).toBe(trigger));
});
it("does not infer paused from pending/error and retries failed IPC", async () => {
 const get=vi.spyOn(api,"getSchedulerStatus").mockRejectedValue(new Error("IPC failed")); mount();
 const user=userEvent.setup(); await user.click(await screen.findByRole("button",{name:"Schedule: Schedule unavailable"}));
 await screen.findByText(/Could not read scheduling status/); expect(screen.queryByRole("button",{name:"Resume automatic runs"})).toBeNull();
 get.mockResolvedValue({...scheduled,enabled_workflows:1,schedules:[],next_run_at:null}); await user.click(screen.getByRole("button",{name:"Refresh status"})); await screen.findByRole("button",{name:"Schedule: Manual only"});
});
it("shows cancellation failure without claiming that a worker stopped, then refreshes completion", async () => {
 let status={...scheduled,running:true,enabled:false,next_run_at:null};
 vi.spyOn(api,"getSchedulerStatus").mockImplementation(async()=>status);
 const cancel=vi.spyOn(api,"cancelScrape").mockRejectedValueOnce(new Error("cancel failed")).mockImplementation(async()=>{status={...status,running:false}; return true;});
 mount();const user=userEvent.setup();await user.click(await screen.findByRole("button",{name:"Schedule: Running · Paused"}));
 await user.click(screen.getByRole("button",{name:"Cancel active run"})); await screen.findByRole("alert"); expect(screen.getByRole("button",{name:"Cancel active run"})).toBeTruthy();
 await user.click(screen.getByRole("button",{name:"Cancel active run"})); await screen.findByRole("button",{name:"Schedule: Paused"});expect(cancel).toHaveBeenCalledTimes(2);expect(screen.queryByRole("button",{name:"Cancel active run"})).toBeNull();
});
it("identifies browser examples without querying or offering scheduler actions",async()=>{
 vi.mocked(isTauri).mockReturnValue(false);const get=vi.spyOn(api,"getSchedulerStatus");mount();const user=userEvent.setup();await user.click(screen.getByRole("button",{name:"Schedule: Desktop required"}));
 expect(screen.getByText(/Browser examples do not run/)).toBeTruthy();expect(get).not.toHaveBeenCalled();expect(screen.queryByRole("button",{name:"Pause automatic runs"})).toBeNull();
});
