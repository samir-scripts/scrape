import { useState, useRef, useEffect, useCallback, type ReactNode } from "react";

interface ThreeColumnProps {
  left: ReactNode;
  center: ReactNode;
  right: ReactNode;
  className?: string;
  initialLeftWidth?: number;
  initialRightWidth?: number;
  minLeftWidth?: number;
  maxLeftWidth?: number;
  minRightWidth?: number;
  maxRightWidth?: number;
}

const DEFAULT_LEFT_WIDTH = 320;
const DEFAULT_RIGHT_WIDTH = 288;
const MIN_LEFT_WIDTH = 220;
const MAX_LEFT_WIDTH = 600;
const MIN_RIGHT_WIDTH = 200;
const MAX_RIGHT_WIDTH = 500;

export function ThreeColumn({
  left,
  center,
  right,
  className = "",
  initialLeftWidth = DEFAULT_LEFT_WIDTH,
  initialRightWidth = DEFAULT_RIGHT_WIDTH,
  minLeftWidth = MIN_LEFT_WIDTH,
  maxLeftWidth = MAX_LEFT_WIDTH,
  minRightWidth = MIN_RIGHT_WIDTH,
  maxRightWidth = MAX_RIGHT_WIDTH,
}: ThreeColumnProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Initialize left pane width from localStorage if available
  const [leftWidth, setLeftWidth] = useState<number>(() => {
    try {
      if (typeof localStorage !== "undefined") {
        const saved = localStorage.getItem("scrape:layout:left_pane_width");
        if (saved) {
          const parsed = parseInt(saved, 10);
          if (!isNaN(parsed) && parsed >= minLeftWidth && parsed <= maxLeftWidth) {
            return parsed;
          }
        }
      }
    } catch {}
    return initialLeftWidth;
  });

  // Initialize right pane width from localStorage if available
  const [rightWidth, setRightWidth] = useState<number>(() => {
    try {
      if (typeof localStorage !== "undefined") {
        const saved = localStorage.getItem("scrape:layout:right_pane_width");
        if (saved) {
          const parsed = parseInt(saved, 10);
          if (!isNaN(parsed) && parsed >= minRightWidth && parsed <= maxRightWidth) {
            return parsed;
          }
        }
      }
    } catch {}
    return initialRightWidth;
  });

  const [isDraggingLeft, setIsDraggingLeft] = useState(false);
  const [isDraggingRight, setIsDraggingRight] = useState(false);

  // Persist widths to localStorage
  useEffect(() => {
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("scrape:layout:left_pane_width", leftWidth.toString());
      }
    } catch {}
  }, [leftWidth]);

  useEffect(() => {
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("scrape:layout:right_pane_width", rightWidth.toString());
      }
    } catch {}
  }, [rightWidth]);

  // Pointer drag for left splitter
  const handleLeftPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDraggingLeft(true);
      const target = e.currentTarget;
      target.setPointerCapture(e.pointerId);

      const onPointerMove = (ev: PointerEvent) => {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        const newWidth = Math.round(ev.clientX - rect.left);
        const clamped = Math.max(minLeftWidth, Math.min(maxLeftWidth, newWidth));
        setLeftWidth(clamped);
      };

      const onPointerUp = (ev: PointerEvent) => {
        setIsDraggingLeft(false);
        try {
          target.releasePointerCapture(ev.pointerId);
        } catch {}
        window.removeEventListener("pointermove", onPointerMove);
        window.removeEventListener("pointerup", onPointerUp);
        window.removeEventListener("pointercancel", onPointerUp);
      };

      window.addEventListener("pointermove", onPointerMove);
      window.addEventListener("pointerup", onPointerUp);
      window.addEventListener("pointercancel", onPointerUp);
    },
    [minLeftWidth, maxLeftWidth]
  );

  // Pointer drag for right splitter
  const handleRightPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDraggingRight(true);
      const target = e.currentTarget;
      target.setPointerCapture(e.pointerId);

      const onPointerMove = (ev: PointerEvent) => {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        const newWidth = Math.round(rect.right - ev.clientX);
        const clamped = Math.max(minRightWidth, Math.min(maxRightWidth, newWidth));
        setRightWidth(clamped);
      };

      const onPointerUp = (ev: PointerEvent) => {
        setIsDraggingRight(false);
        try {
          target.releasePointerCapture(ev.pointerId);
        } catch {}
        window.removeEventListener("pointermove", onPointerMove);
        window.removeEventListener("pointerup", onPointerUp);
        window.removeEventListener("pointercancel", onPointerUp);
      };

      window.addEventListener("pointermove", onPointerMove);
      window.addEventListener("pointerup", onPointerUp);
      window.addEventListener("pointercancel", onPointerUp);
    },
    [minRightWidth, maxRightWidth]
  );

  // Reset widths on double click
  const handleLeftDoubleClick = useCallback(() => {
    setLeftWidth(initialLeftWidth);
  }, [initialLeftWidth]);

  const handleRightDoubleClick = useCallback(() => {
    setRightWidth(initialRightWidth);
  }, [initialRightWidth]);

  // Keyboard navigation for splitters
  const handleLeftKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        setLeftWidth((w) => Math.max(minLeftWidth, w - 10));
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        setLeftWidth((w) => Math.min(maxLeftWidth, w + 10));
      } else if (e.key === "Home") {
        e.preventDefault();
        setLeftWidth(minLeftWidth);
      } else if (e.key === "End") {
        e.preventDefault();
        setLeftWidth(maxLeftWidth);
      }
    },
    [minLeftWidth, maxLeftWidth]
  );

  const handleRightKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        setRightWidth((w) => Math.min(maxRightWidth, w + 10));
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        setRightWidth((w) => Math.max(minRightWidth, w - 10));
      } else if (e.key === "Home") {
        e.preventDefault();
        setRightWidth(minRightWidth);
      } else if (e.key === "End") {
        e.preventDefault();
        setRightWidth(maxRightWidth);
      }
    },
    [minRightWidth, maxRightWidth]
  );

  const isDragging = isDraggingLeft || isDraggingRight;

  return (
    <div
      ref={containerRef}
      className={`flex flex-1 w-full h-full overflow-hidden relative ${
        isDragging ? "select-none cursor-col-resize" : ""
      } ${className}`}
    >
      {/* Left Column: Job List */}
      <aside
        style={{ width: `${leftWidth}px` }}
        className="shrink-0 h-full flex flex-col overflow-y-auto bg-[var(--color-paper)] transition-none"
      >
        {left}
      </aside>

      {/* Left Resizer Handle */}
      <div
        role="separator"
        tabIndex={0}
        aria-orientation="vertical"
        aria-label="Resize job list pane"
        aria-valuenow={leftWidth}
        aria-valuemin={minLeftWidth}
        aria-valuemax={maxLeftWidth}
        title="Drag to resize pane · Double-click to reset"
        onPointerDown={handleLeftPointerDown}
        onDoubleClick={handleLeftDoubleClick}
        onKeyDown={handleLeftKeyDown}
        className={`group relative w-2 -mx-1 shrink-0 h-full cursor-col-resize z-20 flex items-center justify-center focus:outline-none ${
          isDraggingLeft ? "bg-[var(--color-paper-2)]" : "hover:bg-[var(--color-paper-2)]/60"
        }`}
      >
        <div
          className={`w-[1px] h-full transition-colors ${
            isDraggingLeft
              ? "bg-[var(--color-ink)] w-[2px]"
              : "bg-[var(--color-ink)] group-hover:bg-[var(--color-ink)]"
          }`}
        />
        {/* Visual Grab Indicator Pip */}
        <div
          className={`absolute w-1 h-5 bg-[var(--color-ink)] transition-opacity ${
            isDraggingLeft ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          }`}
        />
      </div>

      {/* Center Column: Job Detail */}
      <main className="flex-1 min-w-[320px] h-full flex flex-col overflow-y-auto bg-[var(--color-paper)] p-6">
        {center}
      </main>

      {/* Right Resizer Handle */}
      <div
        role="separator"
        tabIndex={0}
        aria-orientation="vertical"
        aria-label="Resize metadata panel"
        aria-valuenow={rightWidth}
        aria-valuemin={minRightWidth}
        aria-valuemax={maxRightWidth}
        title="Drag to resize panel · Double-click to reset"
        onPointerDown={handleRightPointerDown}
        onDoubleClick={handleRightDoubleClick}
        onKeyDown={handleRightKeyDown}
        className={`group relative w-2 -mx-1 shrink-0 h-full cursor-col-resize z-20 flex items-center justify-center focus:outline-none ${
          isDraggingRight ? "bg-[var(--color-paper-2)]" : "hover:bg-[var(--color-paper-2)]/60"
        }`}
      >
        <div
          className={`w-[1px] h-full transition-colors ${
            isDraggingRight
              ? "bg-[var(--color-ink)] w-[2px]"
              : "bg-[var(--color-ink)] group-hover:bg-[var(--color-ink)]"
          }`}
        />
        {/* Visual Grab Indicator Pip */}
        <div
          className={`absolute w-1 h-5 bg-[var(--color-ink)] transition-opacity ${
            isDraggingRight ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          }`}
        />
      </div>

      {/* Right Column: Metadata / Actions */}
      <aside
        style={{ width: `${rightWidth}px` }}
        className="shrink-0 h-full flex flex-col overflow-y-auto bg-[var(--color-paper-2)]/30 p-4 transition-none"
      >
        {right}
      </aside>
    </div>
  );
}
