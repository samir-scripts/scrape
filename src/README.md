# `src/` — React Frontend Application Root

## Overview

The `src/` directory contains the single-page web application frontend for Scrape. Built with React 18, TypeScript, Tailwind CSS v4, and Visx, it renders a brutalist, keyboard-first desktop user interface styled after NieR:Automata.

---

## Architecture & Data Flow

1. **State Management**: Zustand (`src/lib/store.ts`) manages active navigation views, filter selections, UI drawer states, and modal dialogs.
2. **Server State & Caching**: TanStack React Query handles background polling, caching, and cache invalidation for Tauri IPC command responses.
3. **IPC Client**: `src/lib/api.ts` wraps Tauri's `@tauri-apps/api/core` `invoke()` calls, returning strongly-typed models defined in `src/bindings/`.
4. **Hardware-Accelerated Styling**: Tailwind CSS v4 and CSS variables (`src/styles/index.css`) enforce 0px border radiuses, 1px rules, high-contrast monochrome themes, and custom typography.
5. **Keyboard Control**: `src/hooks/useHotkeys.ts` enables full keyboard navigation across columns, jobs, filters, and actions.

```
┌────────────────────────────────────────────────────────┐
│                   main.tsx / App.tsx                   │
│   (QueryClientProvider, Shell Layout, Hotkey Listener) │
└──────────────────────────┬─────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│  components/  │  │    hooks/     │  │     lib/      │
│ (Views & UI)  │  │  (Hotkeys &   │  │ (Store, API,  │
│               │  │   Debounce)   │  │   Filters)    │
└───────┬───────┘  └───────────────┘  └───────┬───────┘
        │                                     │
        ▼                                     ▼
┌───────────────┐                     ┌───────────────┐
│    styles/    │                     │   bindings/   │
│ (Tailwind &   │                     │  (Rust ts-rs  │
│  Brutalist)   │                     │  Types)       │
└───────────────┘                     └───────────────┘
```

---

## File Manifest

| File | Type | Responsibilities & Implementation |
|---|---|---|
| [`main.tsx`](main.tsx) | Application Entry Point | Bootstraps the React DOM root (`#root`), initializes `QueryClient` with default stale-time and cache policies, and mounts `App.tsx` within `React.StrictMode`. |
| [`App.tsx`](App.tsx) | Root Component | Shell container orchestrating top-level view switching (`DISCOVERED`, `TRACKED`, `SCRAPER`, `METRICS`, `SETTINGS`), global hotkey listeners (`Cmd+K`, view switches, quick search), toast notifications, and global deletion confirmation modals. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`bindings/`](bindings/README.md) | Auto-generated TypeScript type definitions from Rust backend crates. |
| [`components/`](components/README.md) | Reusable brutalist UI widgets, layout containers, filter panels, and top-level view screens. |
| [`hooks/`](hooks/README.md) | Custom React hooks for keyboard navigation and input debouncing. |
| [`lib/`](lib/README.md) | IPC client, Zustand store, client-side filtering algorithms, formatting helpers, and YAML parser. |
| [`styles/`](styles/README.md) | Tailwind CSS v4 styling rules, typography definitions, and NieR brutalist color variables. |
| [`test/`](test/README.md) | Vitest test suites covering components, state stores, navigation math, and filters. |
