# `src/styles` — Global Styles & Design System Rules

## Overview

The `src/styles` directory defines the styling foundation for Scrape. It implements the NieR:Automata-inspired brutalist design system using Tailwind CSS v4, custom CSS variables, and specialized utility classes.

---

## File Manifest

| File | Framework / Format | Description & Key Features |
|---|---|---|
| [`index.css`](index.css) | Tailwind CSS v4 | Core stylesheet defining all semantic theme tokens (`--color-surface`, `--color-surface-raised`, `--color-text`, `--color-text-muted`, `--color-border`, `--color-danger`, `--color-stage-*`, `--color-logo-surface`, `--color-scrollbar-*`), spacing and control height tokens (`--spacing-*`, `--control-height*`), global interface typography (`--font-ui` across 6 bundled options and `--font-code` for YAML/code), Paper (light) and Obsidian (dark) themes, 0px border-radius/shadow resets, and `.bg-dither` patterns. |

---

## Design System Invariants

1. **Strict 0px Border Radius & No Box Shadows**: No rounded corners or drop shadows (`rounded-none`, `shadow-none`), enforced by `scripts/check-style.mjs`.
2. **Hairline 1px Rules**: Crisp, high-contrast borders with one border per meaningful surface.
3. **Semantic Palette & Stage Tokens**: Paper (`#F5F5DC`) and Obsidian (`#262628`) surfaces with Ink (`#1A1A1A`) and Cream (`#FFF6EE`) text, plus semantic stage tokens: Applied lavender (`#C6C2F2`), Interviewing cream (`#FFF6EE` / `#E4C891`), Offer lime (`#D6DC82`), and Rejected cherry (`#D86072`).
4. **Global Typography**: One selected `--font-ui` family applies to all UI surfaces, dialogs, portals, and toasts; uppercase is reserved for short labels and actions while job titles and prose stay in natural case.

Semantic danger/success/warning/information colors now have separate Paper and
Obsidian values. Muted foregrounds were darkened/lightened for text contrast;
global font-smoothing suppression was removed. Token-pair tests check 4.5:1 on
both base surfaces, but do not constitute rendered-state accessibility QA.
JobCard uses independent native buttons, selectable descriptions, a named trash
SVG, and semantic badges. Its removal backend now preserves saved state and uses explicit, reversible
blocking; the control is named Remove.
