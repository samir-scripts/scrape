import {
  useState,
  useRef,
  useEffect,
  useLayoutEffect,
  type ReactNode,
  type MouseEvent,
} from "react";
import { createPortal } from "react-dom";
import { useUIStore } from "../../lib/store";

export {
  calculateTooltipCoordinates,
  getTooltipVisualBounds,
  type TooltipPosition,
  type TooltipCoordinates,
  type BoundingBox,
} from "../../lib/positioning";
import type { TooltipPosition } from "../../lib/positioning";

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  delay?: number;
  position?: TooltipPosition;
  className?: string;
  disabled?: boolean;
  offset?: number;
}

const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

export function Tooltip({
  content,
  children,
  delay = 300,
  position = "top",
  className = "",
  disabled = false,
  offset = 6,
}: TooltipProps) {
  const tooltipsEnabled = useUIStore((state) => state.tooltipsEnabled);
  const [visible, setVisible] = useState(false);
  const [placement, setPlacement] = useState<TooltipPosition>(position);
  const [coords, setCoords] = useState<{ top: number; left: number }>({
    top: 0,
    left: 0,
  });

  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  // If tooltips get globally disabled, dismiss immediately
  useEffect(() => {
    if (!tooltipsEnabled) {
      setVisible(false);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    }
  }, [tooltipsEnabled]);

  const updatePosition = (measuredW = 0, measuredH = 0) => {
    if (!triggerRef.current) return;
    const triggerRect = triggerRef.current.getBoundingClientRect();
    const wWidth = typeof window !== "undefined" ? window.innerWidth : 1024;
    const wHeight = typeof window !== "undefined" ? window.innerHeight : 768;

    const estimatedH = measuredH > 0 ? measuredH : 32;
    const spaceAbove = triggerRect.top - offset - 12;
    const spaceBelow = wHeight - triggerRect.bottom - offset - 12;

    let targetPlacement: TooltipPosition = position;
    if (position === "top" && spaceAbove < estimatedH && spaceBelow > spaceAbove) {
      targetPlacement = "bottom";
    } else if (position === "bottom" && spaceBelow < estimatedH && spaceAbove > spaceBelow) {
      targetPlacement = "top";
    }

    setPlacement(targetPlacement);

    let left = triggerRect.left + triggerRect.width / 2;
    const padding = 12;

    if (measuredW > 0) {
      const halfW = measuredW / 2;
      if (measuredW >= wWidth - padding * 2) {
        left = wWidth / 2;
      } else if (left - halfW < padding) {
        left = padding + halfW;
      } else if (left + halfW > wWidth - padding) {
        left = wWidth - padding - halfW;
      }
    } else if (left < 40) {
      left = 40;
    } else if (left > wWidth - 40) {
      left = wWidth - 40;
    }

    const top =
      targetPlacement === "top"
        ? triggerRect.top - offset
        : triggerRect.bottom + offset;

    setCoords({ top, left });
  };

  const handleMouseEnter = (_e: MouseEvent<HTMLDivElement>) => {
    if (!tooltipsEnabled || disabled || !content) return;
    if (timerRef.current) clearTimeout(timerRef.current);

    timerRef.current = setTimeout(() => {
      updatePosition();
      setVisible(true);
    }, delay);
  };

  const handleMouseLeave = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  };

  // Re-measure accurately once rendered in the portal before paint
  useIsomorphicLayoutEffect(() => {
    if (!visible || !triggerRef.current || !tooltipRef.current) return;
    const tooltipRect = tooltipRef.current.getBoundingClientRect();
    updatePosition(tooltipRect.width, tooltipRect.height);
  }, [visible, position, content]);

  // Close on Escape, scroll, or resize
  useEffect(() => {
    if (!visible) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setVisible(false);
    };

    const handleScroll = () => {
      setVisible(false);
    };

    const handleResize = () => {
      setVisible(false);
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("resize", handleResize);
    };
  }, [visible]);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  return (
    <div
      ref={triggerRef}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      className={`inline-flex items-center ${className}`}
    >
      {children}
      {visible &&
        tooltipsEnabled &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            ref={tooltipRef}
            role="tooltip"
            style={{
              top: `${coords.top}px`,
              left: `${coords.left}px`,
              transform:
                placement === "top"
                  ? "translate(-50%, -100%)"
                  : "translate(-50%, 0)",
            }}
            className="fixed z-[9999] pointer-events-none px-2 py-1 bg-[var(--color-ink)] text-[var(--color-paper)] border border-[var(--color-paper-2)] font-mono text-[10px] uppercase tracking-wider select-none max-w-[280px] sm:max-w-[340px] whitespace-normal break-words text-center shadow-none leading-tight"
          >
            {content}
          </div>,
          document.body
        )}
    </div>
  );
}
