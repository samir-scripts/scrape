# `src/components` — Frontend Component Architecture

## Overview

The `src/components` directory contains the complete modular component hierarchy for Scrape's user interface. All components are styled in accordance with the NieR:Automata brutalist design system (0px border radius, 1px rules, high-contrast monochrome tones, monospaced data grids, and uppercase typography).

---

## Component Hierarchy & Directory Organization

```
src/components/
├── ui/             # Primitive design system controls (buttons, inputs, selects, menus, modals, logos, tags)
├── layout/         # Shell framing, navigation bars, resizable split panes, schedule control, and footer
├── filters/        # Consolidated Filters & sort panel and active filter/sort summary chips
├── jobs/           # Shared JobInspector, sanitized JobDescriptionView, and bottom JobPaginator
├── notifications/  # Notification bell, badge count, and floating alert popover
└── views/          # Top-level application screens
    ├── discovered/ # Unprocessed job feed (card grid, table, shared job inspector)
    ├── tracked/    # Application pipeline (Kanban board, 6-column table list, stage transitions)
    ├── scraper/    # Workflow registry, manual/Run all controls, and run/source diagnostic logs
    ├── metrics/    # Analytics dashboard (application velocity, conversion rates, charts)
    └── settings/   # User preferences (appearance, system, removed/blocked jobs, data management) & workflow editor
```

---

## Design System Invariants

Every component in this directory enforces the following rules:
- **Zero Border Radius & Shadows**: Never use `rounded-*` or `shadow-*` classes (other than `-none`); enforced by `scripts/check-style.mjs`.
- **Explicit 1px Borders**: Dividers and surfaces use hairline 1px borders (`border border-[var(--color-border)]`), with one border per meaningful surface.
- **Semantic Tokens**: Backgrounds use `--color-surface` / `--color-surface-raised`; foregrounds use `--color-text` / `--color-text-muted` in both Paper and Obsidian themes.
- **Restrained Accents**: Stage and status indicators use shared semantic tokens (`--color-stage-applied` lavender, `--color-stage-interviewing` cream, `--color-stage-offer` lime, `--color-stage-rejected` cherry).
- **Keyboard & Target Ergonomics**: Interactive controls are at least 36px high (32px minimum for `IconButton` targets), display visible focus rings, and support keyboard navigation.
