import { useState, useRef, useEffect, useCallback, type ReactNode } from "react";

interface DiscoveredTwoColumnProps {
  left: ReactNode;
  right: ReactNode | null;
  className?: string;
  initialLeftWidth?: number;
  minLeftWidth?: number;
  maxLeftWidth?: number;
}

const DEFAULT_LEFT_WIDTH = 440;
const MIN_LEFT_WIDTH = 320;
const MAX_LEFT_WIDTH = 700;

export function DiscoveredTwoColumn({
  left,
  right,
  className = "",
  initialLeftWidth = DEFAULT_LEFT_WIDTH,
  minLeftWidth = MIN_LEFT_WIDTH,
  maxLeftWidth = MAX_LEFT_WIDTH,
}: DiscoveredTwoColumnProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Initialize left pane width from localStorage if available
  const [leftWidth, setLeftWidth] = useState<number>(() => {
    try {
      if (typeof localStorage !== "undefined") {
        const saved = localStorage.getItem("scrape:layout:discovered_split_width");
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

  const [isDragging, setIsDragging] = useState(false);

  // Persist width to localStorage
  useEffect(() => {
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("scrape:layout:discovered_split_width", leftWidth.toString());
      }
    } catch {}
  }, [leftWidth]);

  // Pointer drag handler for vertical splitter
  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragging(true);
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
        setIsDragging(false);
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

  // Double-click to reset width to default
  const handleDoubleClick = () => {
    setLeftWidth(initialLeftWidth);
  };

  // Keyboard navigation for separator
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      setLeftWidth((w) => Math.max(minLeftWidth, w - 20));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      setLeftWidth((w) => Math.min(maxLeftWidth, w + 20));
    } else if (e.key === "Home") {
      e.preventDefault();
      setLeftWidth(minLeftWidth);
    } else if (e.key === "End") {
      e.preventDefault();
      setLeftWidth(maxLeftWidth);
    }
  };

  // 1-PANE VIEW: When no job is selected (right === null)
  if (!right) {
    return (
      <div
        ref={containerRef}
        className={`flex flex-1 w-full h-full overflow-hidden relative bg-[var(--color-paper)] ${className}`}
      >
        <div className="flex-1 w-full h-full overflow-y-auto">
          <div className="max-w-4xl mx-auto w-full px-4 py-2">
            {left}
          </div>
        </div>
      </div>
    );
  }

  // 2-PANE VIEW: Left card stream + Right consolidated job window with resizable divider
  return (
    <div
      ref={containerRef}
      className={`flex flex-1 w-full h-full overflow-hidden relative select-none ${
        isDragging ? "cursor-col-resize select-none" : ""
      } ${className}`}
    >
      {/* Left Pane: Card Stream */}
      <aside
        style={{ width: `${leftWidth}px` }}
        className="shrink-0 h-full flex flex-col overflow-y-auto bg-[var(--color-paper)] border-r border-[var(--color-ink)] transition-none"
      >
        {left}
      </aside>

      {/* Interactive Resizable Divider */}
      <div
        role="separator"
        tabIndex={0}
        aria-orientation="vertical"
        aria-label="Resize job window"
        aria-valuenow={leftWidth}
        aria-valuemin={minLeftWidth}
        aria-valuemax={maxLeftWidth}
        onPointerDown={handlePointerDown}
        onDoubleClick={handleDoubleClick}
        onKeyDown={handleKeyDown}
        title="Drag to resize · Double-click to reset"
        className={`group relative w-2 -mx-1 shrink-0 h-full cursor-col-resize z-20 flex items-center justify-center focus:outline-none hover:bg-[var(--color-paper-2)]/60 ${
          isDragging ? "bg-[var(--color-paper-2)]/80" : ""
        }`}
      >
        <div
          className={`w-[1px] h-full transition-colors ${
            isDragging
              ? "bg-[var(--color-ink)]"
              : "bg-[var(--color-ink)] group-hover:bg-[var(--color-ink)]"
          }`}
        />
        <div
          className={`absolute w-1 h-6 bg-[var(--color-ink)] transition-opacity ${
            isDragging ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          }`}
        />
      </div>

      {/* Right Pane: Consolidated Job Window */}
      <main className="flex-1 min-w-[380px] h-full flex flex-col overflow-y-auto bg-[var(--color-paper)] select-text">
        {right}
      </main>
    </div>
  );
}
