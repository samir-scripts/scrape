# `src/components/views/tracked` — Application Pipeline Tracking View

## Overview

The `src/components/views/tracked` directory provides tracking and lifecycle management for active job applications. It supports both a drag-and-drop desktop Kanban board and an interactive tabular list across the four pipeline stages: `APPLIED`, `INTERVIEWING`, `OFFER`, and `REJECTED`.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **TrackedView** | [`TrackedView.tsx`](TrackedView.tsx) | Primary controller for the Tracked tab. Manages view switching between Kanban board and List modes, maintains active card selection, and handles keyboard hotkeys (`Shift+Left`/`Shift+Right` for stage shifting, `Enter` to open URL, `Delete` to remove). |
| **KanbanBoard** | [`KanbanBoard.tsx`](KanbanBoard.tsx) | Desktop Kanban container with four columns. Enforces discrete 1px column borders, synchronized headers, independent column scrollbars (`scrollbar-gutter: stable`), drag-and-drop handlers, and spatial keyboard navigation. |
| **KanbanColumn** | [`KanbanColumn.tsx`](KanbanColumn.tsx) | Individual stage column component rendering stage header, count badge, stage color accent, draggable job cards, and brutalist dithered drop zone highlights (`.bg-dither`) during drag-and-drop operations. |
| **TrackedList** | [`TrackedList.tsx`](TrackedList.tsx) | High-density tabular spreadsheet view of tracked applications. Supports column sorting by company, title, stage, application date, and salary. |
| **JobDetailDrawer** | [`JobDetailDrawer.tsx`](JobDetailDrawer.tsx) | Slide-over inspection drawer displaying application history, notes, stage selectors, external link opener, and deletion controls. |
