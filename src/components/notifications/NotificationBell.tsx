import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
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
        <IconButton
          data-control-action="notifications.panel.toggle"
          ref={buttonRef}
          id="notification-bell-btn"
          onClick={toggleNotificationOpen}
          label={tooltipText}
          aria-expanded={isNotificationOpen}
          className={`relative px-2 flex items-center justify-center font-mono text-xs font-bold border transition-none cursor-pointer ${
            isNotificationOpen
              ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
              : "bg-transparent text-[var(--color-ink)] border-transparent hover:border-[var(--color-ink)] hover:text-[var(--color-ink)]"
          }`}
        >
          <Icon name="bell" />

          {/* Brutalist Sharp Square Indicator Pip (0px radius) */}
          {unreadNotificationCount > 0 && (
            <span
              className="absolute top-1 right-1 w-2 h-2 bg-[var(--color-cherry)] border border-[var(--color-paper)] pointer-events-none"
              data-testid="notification-unread-pip"
            />
          )}
        </IconButton>
      </Tooltip>

      <NotificationPopover
        isOpen={isNotificationOpen}
        onClose={() => setNotificationOpen(false)}
        anchorEl={buttonRef.current}
      />
    </>
  );
}
