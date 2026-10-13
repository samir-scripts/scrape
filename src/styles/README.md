# `src/styles` — Global Styles & Design System Rules

## Overview

The `src/styles` directory defines the styling foundation for Scrape. It implements the NieR:Automata-inspired brutalist design system using Tailwind CSS v4, custom CSS variables, and specialized utility classes.

---

## File Manifest

| File | Framework / Format | Description & Key Features |
|---|---|---|
| [`index.css`](index.css) | Tailwind CSS v4 / PostCSS | Core stylesheet defining: custom typography sets (JetBrains Mono, IBM Plex Sans, serif accents), brutalist CSS variables (`--color-paper`, `--color-ink`, `--color-border`, `--color-surface`, `--color-dim`), theme modes (`paper` vs `obsidian`), 0px border-radius resets, 1px borders, scrollbar styling, and `.bg-dither` canvas patterns for drop targets. |

---

## Design System Invariants

1. **Strict 0px Border Radius**: No rounded corners (`rounded-none` or `border-radius: 0`).
2. **Hairline 1px Rules**: Crisp, high-contrast borders between columns, cards, and headers.
3. **Monochrome Dominance**: High contrast base palette (Paper `#F5F5DC` and Ink `#1A1A1A`) with sparse, meaningful signal accents (lime for applied, blue for interviewing, cherry for rejected).
4. **Uppercase Typography**: Headers, labels, tags, and status badges are rendered in uppercase tracking.
