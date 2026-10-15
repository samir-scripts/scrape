# `src/components/views/tracked` — Application Pipeline Tracking View

## Overview

The `src/components/views/tracked` directory provides tracking and lifecycle management for active job applications. It supports both a drag-and-drop desktop Kanban board and an interactive tabular list across the four pipeline stages: `APPLIED`, `INTERVIEWING`, `OFFER`, and `REJECTED`.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **TrackedView** | [`TrackedView.tsx`](TrackedView.tsx) | Primary controller for the Tracked tab. Manages view switching between Kanban board and List modes, maintains active card selection, and handles keyboard hotkeys (`Shift+Left`/`Shift+Right` for stage shifting, `Enter` to open URL, `Delete` to remove). |
| **KanbanBoard** | [`KanbanBoard.tsx`](KanbanBoard.tsx) | Desktop Kanban container with four columns. Enforces discrete 1px column borders, synchronized headers, independent column scrollbars (`scrollbar-gutter: stable`), one pointer-capture controller, edge scrolling and spatial keyboard navigation. Narrow boards retain all columns in a horizontal scroller. |
| **KanbanColumn** | [`KanbanColumn.tsx`](KanbanColumn.tsx) | Individual stage column component rendering stage header, count badge, explicit borders, selectable card text, shared SVG drag handles, Move to controls, pending state and outlined drop targets. |
| **KanbanCard** | [`KanbanCard.tsx`](KanbanCard.tsx) | Shared card presentation for the mounted card, drag placeholder and pointer-following preview. |
| **TrackedList** | [`TrackedList.tsx`](TrackedList.tsx) | High-density tabular spreadsheet view of tracked applications. Supports column sorting by company, title, stage, application date, and salary. |
| **JobDetailDrawer** | [`JobDetailDrawer.tsx`](JobDetailDrawer.tsx) | Responsive shared inspection pane/modal displaying application history, notes, stage selectors, external link opener, and deletion controls. |

## Stage changes

`useKanbanDrag` owns handle-only pointer capture, a movement threshold, hit testing,
animation-frame preview positioning, edge scrolling, Escape/pointer cancellation
and keyboard pick-up/move/confirm. Pointer movement leaves the original card
mounted as a same-size placeholder and shows an inaccessible, noninteractive card
under the pointer. While a dropped status is saving, that preview stays over the
target and its header announces the destination. Reduced-motion settings disable
the settling transition.
Whole cards are not HTML5 draggable; nested controls and text selection retain
normal behavior. The stage picker scrolls to a column rather than hiding targets.
No Tauri drag-drop setting or file-import behavior was changed.

All drag, Move to, inspector, shortcut and Undo actions reach
`useJobStatusChange`. Per-job queues serialize writes and suppress same-stage
requests. Cards remain at the confirmed repository stage while saving and on
failure; the error offers retry. Each successful write refreshes jobs, counts and
history. Queue snapshots are discarded after draining, so refreshed repository
state stays authoritative. Counts on columns describe this page only.

Semantic stage tokens color the Kanban headers/cards and the shared status tags
used in lists and the inspector. Automated browser pointer, keyboard, cancellation
and restart verification passed in both themes. Actual native macOS pointer
acceptance remains unverified; see the section 5 implementation evidence.
