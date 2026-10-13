# `src/components/notifications` — In-App Alert & Notification Center

## Overview

The `src/components/notifications` directory contains components that manage real-time alert delivery, crawl execution summaries, error warnings, and notification history.

---

## Component Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **NotificationBell** | [`NotificationBell.tsx`](NotificationBell.tsx) | Navigation bar bell icon button. Shows a high-contrast badge counter indicating unread notifications and toggles the notification popover. |
| **NotificationPopover** | [`NotificationPopover.tsx`](NotificationPopover.tsx) | Floating alert drawer positioned via `src/lib/positioning.ts`. Displays chronological notification cards (run completions, errors, new job matches), individual mark-read actions, "MARK ALL READ", and "CLEAR ALL" operations. |
