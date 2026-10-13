# `src/hooks` — Custom React Hooks

## Overview

The `src/hooks` directory contains reusable React lifecycle and event hooks designed for performance optimization and desktop keyboard ergonomics.

---

## File Manifest

| File | Hook Name | Purpose & Implementation Details |
|---|---|---|
| [`useDebounce.ts`](useDebounce.ts) | `useDebounce<T>(value, delay)` | Delays state updates until a specified idle duration has elapsed (default: 200–300ms). Used across search bars and range slider filters to debounce rapid user inputs before triggering database queries or heavy filter computations. |
| [`useHotkeys.ts`](useHotkeys.ts) | `useHotkeys(bindings, deps)` | Centralized desktop hotkey engine. Listens for global and contextual keystrokes (`Cmd+K`, `j/k`, `Shift+Left/Right`, `Enter`, `Delete`, `/`, `Escape`). Automatically suppresses key triggers when typing in active text inputs, content-editable fields, or open dialogs, and coordinates with the Zustand store. |
