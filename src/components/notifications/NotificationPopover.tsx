import { useEffect, useRef, useState, useCallback } from "react";
import { createPortal } from "react-dom";
import type { AppNotification } from "../../bindings/AppNotification";
import type { NotificationKind } from "../../bindings/NotificationKind";
import { api } from "../../lib/api";
import { useUIStore } from "../../lib/store";
import {
  calculateAnchoredPopoverCoordinates,
  type FilterDropdownCoordinates,
} from "../../lib/positioning";

interface NotificationPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  anchorEl: HTMLElement | null;
}

export const KIND_CONFIG: Record<
  NotificationKind,
  { label: string; bg: string; text: string; border: string }
> = {
  update: {
    label: "UPDATE",
    bg: "var(--color-signal)",
    text: "#FFFFFF",
    border: "var(--color-signal)",
  },
  news: {
    label: "NEWS",
    bg: "var(--color-lavender)",
    text: "var(--color-ink)",
    border: "var(--color-ink)",
  },
  system: {
    label: "SYSTEM",
    bg: "var(--color-paper-2)",
    text: "var(--color-ink)",
    border: "var(--color-ink-mute)",
  },
  scraper: {
    label: "SCRAPER",
    bg: "var(--color-lime)",
    text: "var(--color-ink)",
    border: "var(--color-ink)",
  },
  error: {
    label: "ERROR",
    bg: "var(--color-cherry)",
    text: "#FFFFFF",
    border: "var(--color-cherry)",
  },
};

export function NotificationPopover({
  isOpen,
  onClose,
  anchorEl,
}: NotificationPopoverProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const [coords, setCoords] = useState<FilterDropdownCoordinates>(() =>
    calculateAnchoredPopoverCoordinates(null, 1024, 768, { align: "right", panelWidth: 400 })
  );

  const [notifications, setNotifications] = useState<AppNotification[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateSuccess, setUpdateSuccess] = useState(false);

  const {
    unreadNotificationCount,
    setUnreadNotificationCount,
    setView,
    setSettingsCategory,
  } = useUIStore();

  const updatePosition = useCallback(() => {
    if (typeof window === "undefined") return;
    const trigger = anchorEl || document.getElementById("notification-bell-btn");
    const rect = trigger ? trigger.getBoundingClientRect() : null;
    setCoords(
      calculateAnchoredPopoverCoordinates(
        rect,
        window.innerWidth,
        window.innerHeight,
        {
          align: "right",
          panelWidth: 420,
          gap: 6,
          viewportPadding: 12,
        }
      )
    );
  }, [anchorEl]);

  const loadNotifications = useCallback(async () => {
    try {
      setIsLoading(true);
      const items = await api.listNotifications(50);
      setNotifications(items);
      const unread = items.filter((item) => !item.read).length;
      setUnreadNotificationCount(unread);
    } catch (err) {
      console.error("Failed to fetch notifications:", err);
    } finally {
      setIsLoading(false);
    }
  }, [setUnreadNotificationCount]);

  useEffect(() => {
    if (!isOpen) return;
    updatePosition();
    loadNotifications();

    const handleResize = () => updatePosition();
    const handleScroll = () => updatePosition();

    window.addEventListener("resize", handleResize);
    window.addEventListener("scroll", handleScroll, true);

    return () => {
      window.removeEventListener("resize", handleResize);
      window.removeEventListener("scroll", handleScroll, true);
    };
  }, [isOpen, updatePosition, loadNotifications]);

  // Click outside listener
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent | PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;

      if (panelRef.current && panelRef.current.contains(target)) {
        return;
      }

      const trigger = anchorEl || document.getElementById("notification-bell-btn");
      if (trigger && trigger.contains(target)) {
        return;
      }

      onClose();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [isOpen, onClose, anchorEl]);

  // ESC key listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true });
    };
  }, [isOpen, onClose]);

  const handleMarkAllRead = async () => {
    try {
      await api.markAllNotificationsRead();
      setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
      setUnreadNotificationCount(0);
    } catch (err) {
      console.error("Failed to mark all as read:", err);
    }
  };

  const handleClearAll = async () => {
    try {
      await api.clearAllNotifications();
      setNotifications([]);
      setUnreadNotificationCount(0);
    } catch (err) {
      console.error("Failed to clear notifications:", err);
    }
  };

  const handleMarkRead = async (id: string) => {
    try {
      await api.markNotificationRead(id);
      setNotifications((prev) =>
        prev.map((n) => (n.id === id ? { ...n, read: true } : n))
      );
      setUnreadNotificationCount(Math.max(0, unreadNotificationCount - 1));
    } catch (err) {
      console.error("Failed to mark notification read:", err);
    }
  };

  const handleInstallUpdate = async () => {
    try {
      setIsUpdating(true);
      setUpdateError(null);
      await api.installAppUpdate();
      setUpdateSuccess(true);
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsUpdating(false);
    }
  };

  const handleOpenSettings = () => {
    onClose();
    setView("settings");
    setSettingsCategory("system");
  };

  if (!isOpen) return null;

  return createPortal(
    <div
      ref={panelRef}
      role="dialog"
      aria-modal="false"
      aria-label="Notification Feed"
      style={{
        position: "fixed",
        top: coords.top,
        left: coords.left,
        width: coords.width,
        maxHeight: coords.maxHeight,
        zIndex: 9999,
      }}
      className="flex flex-col bg-[var(--color-paper)] border border-[var(--color-ink)] shadow-none font-mono text-xs select-none"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 bg-[var(--color-paper-2)] border-b border-[var(--color-ink)] shrink-0">
        <div className="flex items-center gap-2">
          <span className="font-bold tracking-wider text-[var(--color-ink)] uppercase">
            NOTIFICATIONS
          </span>
          {unreadNotificationCount > 0 && (
            <span className="px-1.5 py-0.2 text-xs font-bold bg-[var(--color-cherry)] text-white leading-none">
              {unreadNotificationCount} NEW
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5">
          {unreadNotificationCount > 0 && (
            <button
              onClick={handleMarkAllRead}
              className="px-1.5 py-0.5 text-xs uppercase font-bold border border-[var(--color-ink)] bg-transparent text-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] cursor-pointer"
            >
              READ ALL
            </button>
          )}

          {notifications.length > 0 && (
            <button
              onClick={handleClearAll}
              className="px-1.5 py-0.5 text-xs uppercase font-bold border border-[var(--color-ink-mute)] bg-transparent text-[var(--color-ink-mute)] hover:border-[var(--color-ink)] hover:text-[var(--color-ink)] cursor-pointer"
            >
              CLEAR
            </button>
          )}

          <button
            onClick={onClose}
            aria-label="Close notifications"
            className="w-5 h-5 flex items-center justify-center font-bold text-xs border border-transparent hover:border-[var(--color-ink)] text-[var(--color-ink)] cursor-pointer"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Content Stream */}
      <div className="flex-1 overflow-y-auto divide-y divide-[var(--color-ink-mute)]/30 max-h-[380px]">
        {isLoading && notifications.length === 0 ? (
          <div className="p-6 text-center text-[var(--color-ink-mute)]">
            POLLING NOTIFICATION LOG…
          </div>
        ) : notifications.length === 0 ? (
          <div className="p-8 text-center flex flex-col items-center justify-center gap-2">
            <span className="text-xl text-[var(--color-ink-mute)]">□</span>
            <span className="font-bold tracking-wider text-[var(--color-ink)] uppercase">
              NO NOTIFICATIONS
            </span>
            <span className="text-xs text-[var(--color-ink-mute)]">
              ALL PIPELINES & WORKFLOWS NOMINAL
            </span>
          </div>
        ) : (
          notifications.map((item) => {
            const kindMeta = KIND_CONFIG[item.kind] || KIND_CONFIG.system;
            const isUpdate = item.kind === "update";

            return (
              <div
                key={item.id}
                onClick={() => !item.read && handleMarkRead(item.id)}
                className={`p-3 flex flex-col gap-1.5 transition-none ${
                  item.read
                    ? "bg-transparent opacity-75 hover:opacity-100"
                    : "bg-[var(--color-paper-2)]/40 hover:bg-[var(--color-paper-2)]/70 cursor-pointer"
                }`}
              >
                {/* Meta row: Badge, Timestamp, Unread status */}
                <div className="flex items-center justify-between text-xs">
                  <div className="flex items-center gap-1.5">
                    <span
                      style={{
                        backgroundColor: kindMeta.bg,
                        color: kindMeta.text,
                        borderColor: kindMeta.border,
                      }}
                      className="px-1 py-0.2 border uppercase font-bold leading-none select-none"
                    >
                      {kindMeta.label}
                    </span>
                    <span className="text-[var(--color-ink-mute)]">
                      {item.created_at.slice(0, 16).replace("T", " ")}
                    </span>
                  </div>

                  {!item.read && (
                    <span
                      className="w-2 h-2 bg-[var(--color-cherry)] border border-[var(--color-paper)]"
                      title="Unread"
                    />
                  )}
                </div>

                {/* Title */}
                <div className="font-bold text-xs uppercase tracking-wide text-[var(--color-ink)]">
                  {item.title}
                </div>

                {/* Message */}
                <div className="text-xs text-[var(--color-ink)] whitespace-pre-wrap leading-relaxed">
                  {item.message}
                </div>

                {/* Special Update Action Card */}
                {isUpdate && (
                  <div className="mt-2 pt-2 border-t border-[var(--color-ink-mute)]/30 flex items-center justify-between gap-2">
                    {updateSuccess ? (
                      <span className="text-xs font-bold text-[var(--color-lime)]">
                        UPDATE READY — RESTARTING APPLICATION…
                      </span>
                    ) : updateError ? (
                      <span className="text-xs text-[var(--color-cherry)]">
                        {updateError}
                      </span>
                    ) : (
                      <button
                        disabled={isUpdating}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleInstallUpdate();
                        }}
                        className="px-2 py-1 text-xs font-bold uppercase tracking-wider bg-[var(--color-signal)] text-white border border-[var(--color-signal)] hover:opacity-90 disabled:opacity-50 cursor-pointer"
                      >
                        {isUpdating ? "DOWNLOADING & INSTALLING…" : "[INSTALL UPDATE]"}
                      </button>
                    )}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-[var(--color-paper-2)] border-t border-[var(--color-ink)] text-xs shrink-0">
        <span className="text-[var(--color-ink-mute)] uppercase">
          SCRAPE v0.1.0
        </span>
        <button
          onClick={handleOpenSettings}
          className="text-[var(--color-ink)] hover:underline cursor-pointer uppercase font-bold"
        >
          UPDATE SETTINGS ↗
        </button>
      </div>
    </div>,
    document.body
  );
}
