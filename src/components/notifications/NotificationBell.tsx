import { useEffect, useRef } from "react";
import { useUIStore } from "../../lib/store";
import { api } from "../../lib/api";
import { Tooltip } from "../ui/Tooltip";
import { NotificationPopover } from "./NotificationPopover";

export function NotificationBell() {
  const buttonRef = useRef<HTMLButtonElement>(null);
  const {
    isNotificationOpen,
    setNotificationOpen,
    toggleNotificationOpen,
    unreadNotificationCount,
    setUnreadNotificationCount,
  } = useUIStore();

  useEffect(() => {
    let isMounted = true;
    api
      .countUnreadNotifications()
      .then((count) => {
        if (isMounted) {
          setUnreadNotificationCount(Number(count));
        }
      })
      .catch((err) => {
        console.error("Failed to fetch initial unread notifications count:", err);
      });

    return () => {
      isMounted = false;
    };
  }, [setUnreadNotificationCount]);

  const tooltipText =
    unreadNotificationCount > 0
      ? `NOTIFICATIONS (${unreadNotificationCount} UNREAD)`
      : "NOTIFICATIONS";

  return (
    <>
      <Tooltip content={tooltipText} position="bottom">
        <button
          ref={buttonRef}
          id="notification-bell-btn"
          onClick={toggleNotificationOpen}
          aria-label={tooltipText}
          aria-expanded={isNotificationOpen}
          className={`relative h-7 px-2 flex items-center justify-center font-mono text-xs font-bold border transition-none cursor-pointer ${
            isNotificationOpen
              ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
              : "bg-transparent text-[var(--color-ink)] border-transparent hover:border-[var(--color-ink)] hover:text-[var(--color-ink)]"
          }`}
        >
          {/* Brutalist Angular Polygon Bell */}
          <svg
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="square"
            strokeLinejoin="miter"
            className="shrink-0"
          >
            {/* Top Hanger */}
            <path d="M8 1v2" />
            {/* Body */}
            <path d="M4 5.5L5 3h6l1 2.5v5l1.5 1.5H2.5L4 10.5v-5z" />
            {/* Clapper */}
            <path d="M6.5 13.5h3" />
          </svg>

          {/* Brutalist Sharp Square Indicator Pip (0px radius) */}
          {unreadNotificationCount > 0 && (
            <span
              className="absolute top-1 right-1 w-2 h-2 bg-[var(--color-cherry)] border border-[var(--color-paper)] pointer-events-none"
              data-testid="notification-unread-pip"
            />
          )}
        </button>
      </Tooltip>

      <NotificationPopover
        isOpen={isNotificationOpen}
        onClose={() => setNotificationOpen(false)}
        anchorEl={buttonRef.current}
      />
    </>
  );
}
