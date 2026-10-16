# `src/hooks` — Custom React Hooks

## Overview

The `src/hooks` directory contains reusable React lifecycle and event hooks designed for performance optimization and desktop keyboard ergonomics.

---

## File Manifest

| File | Hook Name | Purpose & Implementation Details |
|---|---|---|
| [`useDebounce.ts`](useDebounce.ts) | `useDebounce<T>(value, delay)` | Delays state updates until a specified idle duration has elapsed (default: 300ms). Used across search inputs and filter controls to debounce rapid user edits before triggering repository queries. |
| [`useHotkeys.ts`](useHotkeys.ts) | `useHotkeys`, `useGlobalHotkeys` | Centralized desktop hotkey engine. Listens for global and contextual keystrokes (`Q/E`, `[/]`, `/`, `F`, `j/k`, `Shift+Left/Right`, `Enter`, `Delete`, `Escape`). Automatically suppresses key triggers when typing in active text inputs, interactive controls, or open dialogs. |
| [`useJobStatusChange.ts`](useJobStatusChange.ts) | `useJobStatusChange(jobs, onChanged)` | Serialized per-job status transition queue shared by Kanban drag-and-drop, Move to menus, `JobInspector`, keyboard shortcuts, and Undo. Keeps cards in their confirmed repository stage until persistence succeeds and refreshes jobs, counts, and history after each write. |
| [`useWorkflowDraft.ts`](useWorkflowDraft.ts) | `useWorkflowDraft(...)` | Single authoritative controller for workflow creation, editing, YAML synchronization, ATS detection, live preview (`preview_workflow`), cancellation, and save (`save_workflow`) across the `Source → Matching → Schedule → Review` wizard. |
