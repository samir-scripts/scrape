import { useEffect, useRef, useState, type PointerEvent, type KeyboardEvent, type RefObject } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import { TRACKED_STAGES } from "../../../lib/constants";

type Drag = { job: Job; target: JobStatus | null; active: boolean; pointer?: number; x: number; y: number; startX: number; startY: number };
// Only the dedicated handle captures pointers. Text, links, controls and OS file drops
// keep their native behavior; no HTML5 drag system competes with this controller.
export function useKanbanDrag(jobs: Job[], move: (id: string, status: JobStatus) => void, root: RefObject<HTMLDivElement | null>) {
 const drag = useRef<Drag | null>(null), [current, setCurrent] = useState<Drag | null>(null);
 const [announcement, announce] = useState("");
 const restore = useRef<string | null>(null);
 const publish = () => setCurrent(drag.current ? { ...drag.current } : null);
 const finish = (commit = false) => {
  const value = drag.current; drag.current = null; publish();
  if (!value) return;
  restore.current = value.job.id;
  if (commit && value.active && value.target && value.target !== value.job.status) {
   move(value.job.id, value.target); announce(`Moving ${value.job.title} to ${value.target}.`);
  } else announce("Movement cancelled. Stage unchanged.");
 };
 const hit = () => {
  const value = drag.current; if (!value) return;
  const target = document.elementFromPoint(value.x, value.y)?.closest<HTMLElement>("[data-kanban-stage]");
  const next = target && root.current?.contains(target) ? target.dataset.kanbanStage as JobStatus : null;
  if (next !== value.target) { value.target = next; publish(); }
 };
 useEffect(() => {
  if (!current || current.pointer === undefined || !current.active) return;
  let frame: number;
  const scroll = () => {
   const value = drag.current, board = root.current;
   if (!value || !board) return;
   const box = board.getBoundingClientRect();
   const edge = (position: number, min: number, max: number) => position < min + 40 ? -12 : position > max - 40 ? 12 : 0;
   if (value.y >= box.top && value.y <= box.bottom) board.scrollLeft += edge(value.x, box.left, box.right);
   const column = document.elementFromPoint(value.x, value.y)?.closest("[data-kanban-stage]")?.querySelector<HTMLElement>("[data-stage-scroll]");
   if (column) { const bounds = column.getBoundingClientRect(); column.scrollTop += edge(value.y, bounds.top, bounds.bottom); }
   hit(); frame = requestAnimationFrame(scroll);
  };
  frame = requestAnimationFrame(scroll); return () => cancelAnimationFrame(frame);
 }, [current?.pointer, current?.active]);
 useEffect(() => {
  const cancel = () => finish();
  const escape = (event: globalThis.KeyboardEvent) => { if (event.key === "Escape" && drag.current) { event.preventDefault(); event.stopImmediatePropagation(); finish(); } };
  window.addEventListener("blur", cancel); window.addEventListener("keydown", escape, true);
  return () => { window.removeEventListener("blur", cancel); window.removeEventListener("keydown", escape, true); };
 }, []);
 useEffect(() => {
  if (drag.current && !jobs.some(job => job.id === drag.current?.job.id)) finish();
  if (restore.current) {
   const handle = document.getElementById(`kanban-card-${restore.current}`)?.querySelector<HTMLButtonElement>("[data-drag-handle]");
   if (handle) { handle.focus({ preventScroll: true }); restore.current = null; }
  }
 }, [jobs]);
 return { current, announcement, handlers: (job: Job) => ({
  onPointerDown: (event: PointerEvent<HTMLButtonElement>) => {
   if (event.button !== 0) return;
   event.preventDefault(); event.currentTarget.focus(); event.currentTarget.setPointerCapture(event.pointerId);
   drag.current = { job, target: job.status, active: false, pointer: event.pointerId, x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY }; publish();
  },
  onPointerMove: (event: PointerEvent<HTMLButtonElement>) => {
   const value = drag.current; if (!value || value.pointer !== event.pointerId) return;
   value.x = event.clientX; value.y = event.clientY;
   if (!value.active && Math.hypot(value.x - value.startX, value.y - value.startY) >= 6) { value.active = true; publish(); }
   if (value.active) hit();
  },
  onPointerUp: (event: PointerEvent<HTMLButtonElement>) => {
   if (drag.current?.pointer !== event.pointerId) return;
   hit(); finish(true); event.currentTarget.releasePointerCapture(event.pointerId);
  },
  onPointerCancel: () => finish(), onLostPointerCapture: () => { if (drag.current?.pointer !== undefined) finish(); },
  onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => {
   if (![" ", "Enter", "ArrowLeft", "ArrowRight"].includes(event.key)) return;
   event.preventDefault(); event.stopPropagation();
   if (!drag.current && (event.key === " " || event.key === "Enter")) {
    drag.current = { job, active: true, target: job.status, x: 0, y: 0, startX: 0, startY: 0 }; publish();
    announce(`Moving ${job.title}. Use left/right arrows, Enter to confirm or Escape to cancel.`);
   } else if (drag.current?.pointer === undefined && drag.current) {
    if (event.key === "Enter" || event.key === " ") finish(true);
    else { const index = TRACKED_STAGES.indexOf(drag.current.target || job.status); drag.current.target = TRACKED_STAGES[Math.max(0, Math.min(TRACKED_STAGES.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)))]; publish(); document.getElementById(`kanban-stage-${drag.current.target}`)?.scrollIntoView({ block: "nearest", inline: "nearest" }); announce(`Target ${drag.current.target}`); }
   }
  },
 }) };
}
