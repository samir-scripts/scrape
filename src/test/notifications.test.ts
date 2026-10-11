import { describe, it, expect, beforeEach } from "vitest";
import { useUIStore } from "../lib/store";
import {
  calculateAnchoredPopoverCoordinates,
  calculateFilterDropdownCoordinates,
} from "../lib/positioning";
import { api, DEFAULT_SETTINGS } from "../lib/api";

describe("Notifications & Updater Suite", () => {
  beforeEach(() => {
    useUIStore.setState({
      isNotificationOpen: false,
      unreadNotificationCount: 0,
    });
  });

  describe("UI Store Notification State", () => {
    it("initializes with notification popover closed and 0 unread", () => {
      const state = useUIStore.getState();
      expect(state.isNotificationOpen).toBe(false);
      expect(state.unreadNotificationCount).toBe(0);
    });

    it("toggles notification popover open and closed", () => {
      useUIStore.getState().toggleNotificationOpen();
      expect(useUIStore.getState().isNotificationOpen).toBe(true);

      useUIStore.getState().toggleNotificationOpen();
      expect(useUIStore.getState().isNotificationOpen).toBe(false);
    });

    it("explicitly sets notification popover open state", () => {
      useUIStore.getState().setNotificationOpen(true);
      expect(useUIStore.getState().isNotificationOpen).toBe(true);

      useUIStore.getState().setNotificationOpen(false);
      expect(useUIStore.getState().isNotificationOpen).toBe(false);
    });

    it("updates unread notification count correctly", () => {
      useUIStore.getState().setUnreadNotificationCount(5);
      expect(useUIStore.getState().unreadNotificationCount).toBe(5);

      useUIStore.getState().setUnreadNotificationCount(0);
      expect(useUIStore.getState().unreadNotificationCount).toBe(0);
    });
  });

  describe("Popover Geometric Positioning Engine", () => {
    it("right-aligns popover flush with trigger right boundary", () => {
      // Trigger at top right: right = 1000, width = 40, top = 10, bottom = 38
      const triggerRect = {
        top: 10,
        bottom: 38,
        left: 960,
        right: 1000,
        width: 40,
      };

      const coords = calculateAnchoredPopoverCoordinates(
        triggerRect,
        1024,
        768,
        {
          align: "right",
          panelWidth: 400,
          gap: 6,
          viewportPadding: 12,
        }
      );

      // Top should be bottom + gap (38 + 6 = 44)
      expect(coords.top).toBe(44);
      // Width should be 400
      expect(coords.width).toBe(400);
      // Left should be right - width (1000 - 400 = 600)
      expect(coords.left).toBe(600);
      // Right edge of popover (left + width) equals trigger right (1000)
      expect(coords.left + coords.width).toBe(triggerRect.right);
    });

    it("clamps right edge within viewport when trigger exceeds screen width", () => {
      const triggerRect = {
        top: 10,
        bottom: 38,
        left: 1010,
        right: 1050,
        width: 40,
      };

      const coords = calculateAnchoredPopoverCoordinates(
        triggerRect,
        1024,
        768,
        {
          align: "right",
          panelWidth: 400,
          gap: 6,
          viewportPadding: 12,
        }
      );

      // Max left is windowWidth (1024) - panelWidth (400) - padding (12) = 612
      expect(coords.left).toBe(612);
      expect(coords.left + coords.width).toBe(1024 - 12);
    });

    it("clamps left edge on narrow mobile displays", () => {
      const triggerRect = {
        top: 10,
        bottom: 38,
        left: 200,
        right: 240,
        width: 40,
      };

      const coords = calculateAnchoredPopoverCoordinates(
        triggerRect,
        320,
        600,
        {
          align: "right",
          panelWidth: 400,
          gap: 4,
          viewportPadding: 10,
        }
      );

      // On 320px viewport with padding 10, width is clamped to 300
      expect(coords.width).toBe(300);
      // Left edge should not be less than padding (10)
      expect(coords.left).toBe(10);
    });

    it("falls back gracefully when trigger rect is null", () => {
      const coords = calculateAnchoredPopoverCoordinates(null, 1024, 768, {
        align: "right",
        panelWidth: 400,
      });

      expect(coords.top).toBe(48);
      expect(coords.left).toBe(1024 - 400 - 12);
      expect(coords.width).toBe(400);
    });

    it("maintains backward compatibility with calculateFilterDropdownCoordinates", () => {
      const triggerRect = {
        top: 40,
        bottom: 72,
        left: 100,
        right: 180,
        width: 80,
      };

      const coords = calculateFilterDropdownCoordinates(triggerRect, 1024, 768);
      expect(coords.top).toBe(76); // 72 + 4 gap
      expect(coords.left).toBe(100); // left-aligned with trigger
      expect(coords.width).toBe(420);
    });
  });

  describe("API Client Notifications & Updater Methods", () => {
    it("lists dev notifications and marks items as read", async () => {
      const items = await api.listNotifications();
      expect(Array.isArray(items)).toBe(true);
      expect(items.length).toBeGreaterThan(0);

      const firstId = items[0].id;
      const readItem = await api.markNotificationRead(firstId);
      expect(readItem.read).toBe(true);

      const unreadCount = await api.countUnreadNotifications();
      expect(unreadCount).toBe(0);
    });

    it("marks all notifications read and clears all notifications", async () => {
      await api.markAllNotificationsRead();
      let unread = await api.countUnreadNotifications();
      expect(unread).toBe(0);

      await api.clearAllNotifications();
      const clearedList = await api.listNotifications();
      expect(clearedList).toEqual([]);

      unread = await api.countUnreadNotifications();
      expect(unread).toBe(0);
    });

    it("queries update endpoint in development mode without exceptions", async () => {
      const update = await api.checkAppUpdate();
      expect(update).toHaveProperty("available");
      expect(update).toHaveProperty("current_version");
      expect(update.current_version).toBe("0.1.0");
    });

    it("invokes install app update without throwing", async () => {
      await expect(api.installAppUpdate()).resolves.toBeUndefined();
    });

    it("creates an error notification and retrieves it from notifications list", async () => {
      const created = await api.createNotification({
        kind: "error",
        title: "PARSE / SPECIFICATION ISSUE",
        message: "Invalid YAML syntax: bad indentation at line 4",
        action_label: "VIEW WORKFLOW",
        action_payload: "Stripe",
      });

      expect(created.id).toBeTruthy();
      expect(created.kind).toBe("error");
      expect(created.title).toBe("PARSE / SPECIFICATION ISSUE");
      expect(created.message).toContain("bad indentation");
      expect(created.action_label).toBe("VIEW WORKFLOW");
      expect(created.action_payload).toBe("Stripe");
      expect(created.read).toBe(false);

      const list = await api.listNotifications();
      const found = list.find((n) => n.id === created.id);
      expect(found).toBeDefined();
      expect(found?.kind).toBe("error");
    });

    it("invokes openTerminalConsole in development without throwing", async () => {
      await expect(api.openTerminalConsole()).resolves.toBeUndefined();
    });
  });

  describe("Notification Kinds & Badge Styling Configuration", () => {
    it("configures error notifications with cherry brutalist accent", async () => {
      const { KIND_CONFIG } = await import(
        "../components/notifications/NotificationPopover"
      );
      expect(KIND_CONFIG.error).toBeDefined();
      expect(KIND_CONFIG.error.label).toBe("ERROR");
      expect(KIND_CONFIG.error.bg).toBe("var(--color-cherry)");
      expect(KIND_CONFIG.error.text).toBe("#FFFFFF");
      expect(KIND_CONFIG.error.border).toBe("var(--color-cherry)");
    });
  });

  describe("Settings Update Channel & Automation Defaults", () => {
    it("has expected default updater settings", () => {
      expect(DEFAULT_SETTINGS.app_auto_update).toBe(true);
      expect(DEFAULT_SETTINGS.app_update_channel).toBe("stable");
      expect(DEFAULT_SETTINGS.app_last_update_check).toBeNull();
    });
  });
});
