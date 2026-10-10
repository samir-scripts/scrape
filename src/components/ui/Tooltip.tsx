import {
  useState,
  useRef,
  useEffect,
  type ReactNode,
  type MouseEvent,
} from "react";
import { createPortal } from "react-dom";

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  delay?: number;
  position?: "top" | "bottom";
  className?: string;
  disabled?: boolean;
}

export function calculateTooltipCoordinates(
  rect: { top: number; bottom: number; left: number; width: number },
  position: "top" | "bottom" = "top"
): { top: number; left: number } {
  let top = 0;
  if (position === "top") {
    top = rect.top - 28; // standard 24px height + 4px gap
  } else {
    top = rect.bottom + 4;
  }

  // Center horizontally relative to trigger
  let left = rect.left + rect.width / 2;

  // Safety margins
  if (top < 4) top = rect.bottom + 4;
  if (left < 40) left = 40;

  return { top, left };
}

export function Tooltip({
  content,
  children,
  delay = 300,
  position = "top",
  className = "",
  disabled = false,
}: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number }>({
    top: 0,
    left: 0,
  });

  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  const calculatePosition = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    setCoords(calculateTooltipCoordinates(rect, position));
  };

  const handleMouseEnter = (_e: MouseEvent<HTMLDivElement>) => {
    if (disabled || !content) return;
    if (timerRef.current) clearTimeout(timerRef.current);

    timerRef.current = setTimeout(() => {
      calculatePosition();
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

  // Close on Escape or scroll
  useEffect(() => {
    if (!visible) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setVisible(false);
    };

    const handleScroll = () => {
      setVisible(false);
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("scroll", handleScroll, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("scroll", handleScroll, true);
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
        typeof document !== "undefined" &&
        createPortal(
          <div
            ref={tooltipRef}
            role="tooltip"
            style={{
              top: `${coords.top}px`,
              left: `${coords.left}px`,
              transform: "translateX(-50%)",
            }}
            className="fixed z-[9999] pointer-events-none px-2 py-0.5 bg-[var(--color-ink)] text-[var(--color-paper)] border border-[var(--color-paper-2)] font-mono text-[10px] uppercase tracking-wider select-none whitespace-nowrap shadow-none"
          >
            {content}
          </div>,
          document.body
        )}
    </div>
  );
}
