# `src/components` — Frontend Component Architecture

## Overview

The `src/components` directory contains the complete modular component hierarchy for Scrape's user interface. All components are styled in accordance with the NieR:Automata brutalist design system (0px border radius, 1px rules, high-contrast monochrome tones, monospaced data grids, and uppercase typography).

---

## Component Hierarchy & Directory Organization

```
src/components/
├── ui/             # Primitive design system controls (buttons, inputs, modals, tags, sliders)
├── layout/         # Shell framing, navigation bars, multi-column panes, and footer status bar
├── filters/        # Multi-faceted job filter controls (role, salary, location, eligibility)
├── notifications/  # Notification bell, badge count, and floating alert popover
└── views/          # Top-level application screens
    ├── discovered/ # Unprocessed job feed (card grid, split pane, job detail inspection)
    ├── tracked/    # Application pipeline (Kanban board, table list, stage transitions)
    ├── scraper/    # Crawler control room (source configs, run triggers, live telemetry)
    ├── metrics/    # Analytics dashboard (application velocity, conversion rates, charts)
    └── settings/   # User preferences (sources, schedule, appearance, system, logs)
```

---

## Design System Invariants

Every component in this directory enforces the following rules:
- **Zero Border Radius**: Never use `rounded-*` classes; all corners are sharp (`rounded-none`).
- **Explicit 1px Borders**: Dividers and cards use hairline 1px borders (`border border-[var(--color-border)]`).
- **High Contrast Base**: Backgrounds use `--color-paper` or `--color-surface`; foregrounds use `--color-ink`.
- **Minimalist Accents**: Color is reserved for state indicators (e.g. stage colors: lime, signal blue, cherry).
- **Keyboard Ergonomics**: Interactive components display visible focus rings and support key navigation.
