import { afterEach, expect, it, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { setupAppEventListeners } from "../lib/events";
import { useUIStore } from "../lib/store";
import type { RetentionStatus } from "../bindings/RetentionStatus";
const handlers = new Map<string, (event: {payload:unknown})=>void>();
vi.mock("@tauri-apps/api/core",()=>({isTauri:()=>true}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async(name:string,handler:(event:{payload:unknown})=>void)=>{handlers.set(name,handler);return ()=>handlers.delete(name);})}));
afterEach(()=>{handlers.clear();useUIStore.getState().setScanning(false,"");});
it("invalidates canonical schedule status after source create/update/toggle/delete, settings, run start, source completion and cancellation/completion",async()=>{
 const client=new QueryClient();const invalidate=vi.spyOn(client,"invalidateQueries");const stop=await setupAppEventListeners(client);
 for(const event of ["sources://changed","settings://changed","scrape://started","scrape://source_finished","scrape://finished"]){
  invalidate.mockClear();handlers.get(event)!({payload:{source_id:"a"}});expect(invalidate).toHaveBeenCalledWith({queryKey:["scheduler_status"]});
 }
 expect(useUIStore.getState().isScanning).toBe(false);stop();expect(handlers.size).toBe(0);
});

it("refreshes retention status, lists, totals, metrics and stale selection after cleanup removes postings",async()=>{
 const client=new QueryClient();const invalidate=vi.spyOn(client,"invalidateQueries");const stop=await setupAppEventListeners(client);
 const revision=useUIStore.getState().dataRevision;
 useUIStore.setState({selectedJobId:"expired",drawerJobId:"expired",selectedJobIds:["expired"],undoStack:[{jobId:"expired",previousStatus:"viewed"}]});
 const status:RetentionStatus={last_attempt_at:"2026-10-15T12:00:00Z",last_success_at:"2026-10-15T12:00:00Z",last_deleted_records:2,consecutive_failures:0,last_error:null};
 handlers.get("retention://updated")!({payload:status});
 await new Promise<void>(resolve=>setTimeout(resolve,0));
 expect(useUIStore.getState().dataRevision).toBe(revision+1);
 expect(useUIStore.getState().selectedJobId).toBeNull();expect(useUIStore.getState().drawerJobId).toBeNull();expect(useUIStore.getState().selectedJobIds).toEqual([]);expect(useUIStore.getState().undoStack).toEqual([]);
 for(const key of ["retention_status","jobs","job_counts","job_status_events","metrics","sources","scheduler_status"]) expect(invalidate).toHaveBeenCalledWith({queryKey:[key]});
 stop();
});

it.each(["success", "failed", "cancelled"])("refreshes every saved-job page and totals on %s completion without changing view filters", async status => {
 const client=new QueryClient();
 const stop=await setupAppEventListeners(client);
 const keys=[["jobs","discovered-filtered",0],["jobs","discovered-filtered",20],["jobs","tracked",0],["job_counts"]];
 for(const key of keys)client.setQueryData(key,{items:[],total:0});
 const before=useUIStore.getState().filterSpec;
 handlers.get("scrape://finished")!({payload:{status}});
 for(const key of keys)expect(client.getQueryState(key)?.isInvalidated).toBe(true);
 expect(useUIStore.getState().filterSpec).toEqual(before);
 expect(useUIStore.getState().isScanning).toBe(false);
 stop();client.clear();
});
