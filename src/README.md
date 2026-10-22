# `src/` — React Frontend Application Root

## Overview

The `src/` directory contains the single-page web application frontend for Scrape. Built with React 19, TypeScript, Tailwind CSS v4, Headless UI, and Visx, it renders a brutalist, keyboard-first desktop user interface styled after NieR:Automata.

---

## Architecture & Data Flow

1. **State Management**: Zustand (`src/lib/store.ts`) manages active navigation views, filter/sort selections, UI drawer states, and modal dialogs.
2. **Server State & Caching**: TanStack React Query handles background polling, repository page queries, caching, and cache invalidation for Tauri IPC command responses.
3. **IPC Client**: `src/lib/api.ts` wraps Tauri's `@tauri-apps/api/core` `invoke()` calls, returning strongly-typed models defined in `src/bindings/` (with an explicit `?demo=1` recorded-example mode outside Tauri).
4. **Hardware-Accelerated Styling**: Tailwind CSS v4 and semantic CSS variables (`src/styles/index.css`) enforce 0px border radiuses, 1px rules, high-contrast Paper/Obsidian themes, and one global `--font-ui` family (`--font-code` for YAML/code).
5. **Keyboard Control**: `src/hooks/useHotkeys.ts` enables full keyboard navigation across columns, jobs, filters, and actions without firing inside active inputs or dialogs.

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
│ (Views & UI)  │  │ (Draft, Queue │  │ (Store, API,  │
│               │  │  & Hotkeys)   │  │  Formatters)  │
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
| [`App.tsx`](App.tsx) | Root Component | Shell container orchestrating top-level view switching (`DISCOVERED`, `TRACKED`, `WORKFLOWS`, `METRICS`, `SETTINGS`), repository job pagination and filter/sort queries, global hotkey listeners, toast notifications, soft-removal confirmation dialogs with Undo, and the desktop-required gate vs. explicit `?demo=1` recorded mode. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`bindings/`](bindings/README.md) | Auto-generated TypeScript type definitions from Rust backend crates (`npm run gen:types`). |
| [`components/`](components/README.md) | Reusable brutalist UI primitives, including searchable multi-selects and keyword autocomplete, layout containers, filter/sort panels, shared job inspector/paginator, and top-level view screens. |
| [`hooks/`](hooks/README.md) | Custom React hooks for workflow draft management, including atomic company-board attachment, serialized status transitions (`useJobStatusChange`), keyboard navigation (`useHotkeys`), and input debouncing (`useDebounce`). |
| [`lib/`](lib/README.md) | IPC client, Zustand store, local company logo cache, formatting helpers, YAML/workflow import bridges, and demo-only filtering/recorded data. |
| [`styles/`](styles/README.md) | Tailwind CSS v4 styling rules, global typography tokens, and NieR brutalist semantic theme variables. |
| [`test/`](test/README.md) | Vitest + React Testing Library suites covering components, user interactions, state stores, navigation, and contracts. |
