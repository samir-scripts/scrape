import { useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import { api } from "../lib/api";
import { useToastStore } from "../lib/toast";

// Stage changes from drag, Move to, inspector, shortcuts and Undo share this queue.
// Cards stay in their confirmed stage until persistence succeeds, so failure never
// requires reverting a newer transition or restoring an obsolete query snapshot.
export function useJobStatusChange(jobs: Job[], onChanged: (previous: Job, next: Job, recordUndo: boolean) => void) {
 const client = useQueryClient(), jobsRef = useRef(jobs), changed = useRef(onChanged);
 jobsRef.current = jobs; changed.current = onChanged;
 const tails = useRef(new Map<string, Promise<unknown>>()), counts = useRef(new Map<string, number>());
 const confirmed = useRef(new Map<string, Job>());
 const [pendingIds, setPending] = useState<ReadonlySet<string>>(new Set());
 const changeStatus = (id: string, status: JobStatus, recordUndo = true): Promise<Job | undefined> => {
  counts.current.set(id, (counts.current.get(id) || 0) + 1); setPending(new Set(counts.current.keys()));
  const task = (tails.current.get(id) || Promise.resolve()).catch(() => {}).then(async () => {
   const previous = confirmed.current.get(id) || jobsRef.current.find(job => job.id === id) || await api.getJob(id);
   if (previous.status === status) return previous;
   const next = await api.updateJobStatus(id, status);
   confirmed.current.set(id, next); changed.current(previous, next, recordUndo);
   return next;
  }).catch(error => {
   useToastStore.getState().addToast({ type: "error", title: "STAGE CHANGE FAILED", duration: 0,
    message: `${String(error)}. The saved stage was kept.`, action: { label: "Retry stage change", run: () => { void changeStatus(id, status, recordUndo); } } });
   return undefined;
  }).finally(async () => {
   await Promise.all(["jobs", "job_counts", "job_status_events"].map(key => client.invalidateQueries({ queryKey: [key] })));
   const remaining = (counts.current.get(id) || 1) - 1;
   if (remaining) counts.current.set(id, remaining); else { counts.current.delete(id); tails.current.delete(id); confirmed.current.delete(id); }
   setPending(new Set(counts.current.keys()));
  });
  tails.current.set(id, task); return task;
 };
 return { changeStatus, pendingIds };
}
