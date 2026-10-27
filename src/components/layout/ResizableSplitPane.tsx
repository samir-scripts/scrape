import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { recordControlAction } from "../../lib/controlActivity";

const MIN_PRIMARY = 440, MIN_SECONDARY = 360, HANDLE = 12, DEFAULT_RATIO = 0.6;
export const JOB_SPLIT_PREFERENCE = "scrape:job_split:v1";
export interface SplitLayout { inline: boolean; reset: () => void }
function readRatio(key: string) {
  try {
    const value = JSON.parse(localStorage.getItem(key) || "null");
    if (value?.version === 1 && Number.isFinite(value.ratio) && value.ratio > 0 && value.ratio < 1) return value.ratio as number;
  } catch { /* An unavailable/corrupt optional preference must not prevent opening jobs. */ }
  return DEFAULT_RATIO;
}
export function ResizableSplitPane({ primary, secondary, preferenceKey = JOB_SPLIT_PREFERENCE }: {
  primary: ReactNode; secondary?: (layout: SplitLayout) => ReactNode; preferenceKey?: string;
}) {
  const root = useRef<HTMLDivElement>(null), handle = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(() => typeof window === "undefined" ? 1280 : window.innerWidth);
  const [preferred, setPreferred] = useState(() => readRatio(preferenceKey));
  const preferredRef = useRef(preferred); preferredRef.current = preferred;
  const dragging = useRef<{ id: number; x: number; ratio: number; original: number } | null>(null);
  const id = useId();
  const inline = width >= MIN_PRIMARY + MIN_SECONDARY + HANDLE;
  const available = Math.max(1, width - HANDLE);
  const min = MIN_PRIMARY / available, max = 1 - MIN_SECONDARY / available;
  const clamp = (value: number) => Math.max(min, Math.min(max, value));
  const ratio = inline ? clamp(preferred) : preferred;
  const value = (number: number) => Number((number * 100).toFixed(2));
  useLayoutEffect(() => {
    const element = root.current!;
    const measure = () => setWidth(element.getBoundingClientRect().width || window.innerWidth);
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(element); window.addEventListener("resize", measure);
    return () => { observer?.disconnect(); window.removeEventListener("resize", measure); };
  }, []);
  const persist = (next: number) => {
    preferredRef.current = next; setPreferred(next);
    try { localStorage.setItem(preferenceKey, JSON.stringify({ version: 1, ratio: next })); } catch { /* Resize still works in restricted storage. */ }
  };
  const reset = () => { recordControlAction("layout.split.reset", "resize"); persist(DEFAULT_RATIO); };
  const cancel = () => { if (dragging.current) { recordControlAction("layout.split.cancel", "resize"); setPreferred(dragging.current.original); dragging.current = null; } };
  return <div ref={root} data-resizable-split className="flex flex-1 min-h-0 min-w-0 overflow-hidden">
    <div id={id} role="region" aria-label="Job results" className="flex min-h-0 min-w-0" style={{ flex: secondary && inline ? `${ratio} 1 0` : "1 1 0" }}>{primary}</div>
    <div ref={handle} role="separator" data-control-action="layout.split.resize" data-control-semantic aria-label="Job results width" aria-controls={id} aria-orientation="vertical"
      aria-valuemin={value(min)} aria-valuemax={value(max)} aria-valuenow={value(ratio)} aria-valuetext={`${Math.round(ratio * 100)}% job results`}
      tabIndex={0} title="Drag to resize. Arrow keys resize; Home/End reach limits."
      className={`${secondary && inline ? "flex" : "hidden"} split-handle shrink-0 items-center justify-center cursor-col-resize touch-none select-none`}
      style={{ width: HANDLE }}
      onPointerDown={event => {
        if (event.button !== 0) return;
        event.preventDefault(); event.currentTarget.focus();
        dragging.current = { id: event.pointerId, x: event.clientX, ratio, original: preferred };
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={event => {
        const drag = dragging.current;
        if (!drag || drag.id !== event.pointerId) return;
        const next = clamp(drag.ratio + (event.clientX - drag.x) / available);
        preferredRef.current = next; setPreferred(next);
      }}
      onPointerUp={event => {
        if (dragging.current?.id !== event.pointerId) return;
        recordControlAction("layout.split.resize", "resize");
        dragging.current = null; persist(preferredRef.current);
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onPointerCancel={cancel} onLostPointerCapture={cancel}
      onKeyDown={event => {
        if (event.key === "Escape" && dragging.current) { event.preventDefault(); event.stopPropagation(); cancel(); return; }
        const next = event.key === "ArrowLeft" ? ratio - 0.02 : event.key === "ArrowRight" ? ratio + 0.02 : event.key === "Home" ? min : event.key === "End" ? max : undefined;
        if (next !== undefined) { event.preventDefault(); event.stopPropagation(); recordControlAction("layout.split.resize", "resize"); persist(clamp(next)); }
      }}><span aria-hidden="true" className="h-full border-l border-[var(--color-ink-mute)]" /></div>
    <div className="flex min-h-0 min-w-0" style={{ flex: secondary && inline ? `${1 - ratio} 1 0` : "0 0 0" }}>
      {secondary?.({ inline, reset })}
    </div>
  </div>;
}
