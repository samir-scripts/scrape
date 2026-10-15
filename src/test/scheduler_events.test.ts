import { afterEach, expect, it, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { setupAppEventListeners } from "../lib/events";
import { useUIStore } from "../lib/store";
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
