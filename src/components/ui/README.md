# `src/components/ui` — Reusable Brutalist UI Primitives

## Overview

The `src/components/ui` directory contains atomic, reusable UI components implementing Scrape's NieR:Automata brutalist design system. Each primitive enforces square geometry, thin rules, restrained accents, and the user-selected interface font. Uppercase is reserved for short actions and labels.

---

## Component Manifest

| Component | File | Responsibilities & Props |
|---|---|---|
| **Icon** | [`Icon.tsx`](Icon.tsx) | Original SVG paths on a shared 24-unit grid, with 14/16/20px sizes, `currentColor`, square caps, and decorative accessibility semantics. |
| **IconButton** | [`IconButton.tsx`](IconButton.tsx) | Named, focusable controls with a shared 32px minimum target, disabled semantics, forwarded refs, and outlined/danger/inverse variants. |
| **Button** | [`Button.tsx`](Button.tsx) | Brutalist button supporting variants (`primary`, `secondary`, `ghost`, `danger`), sizes (`sm`, `md`, `lg`), high-contrast inverted active states, and keyboard focus outlines. |
| **ClockPicker** | [`ClockPicker.tsx`](ClockPicker.tsx) | Interactive 24-hour circular analog clock face component. Allows users to click or drag radial hands to pick crawl execution hours and minutes, updating cron expressions. |
| **CompanyLogo** | [`CompanyLogo.tsx`](CompanyLogo.tsx) | Company favicon image renderer using Google favicon proxy URLs with automatic fallback to a monospaced initial box on network errors or missing logos. |
| **DeleteJobModal** | [`DeleteJobModal.tsx`](DeleteJobModal.tsx) | Soft-removal confirmation preserving saved state, with an initially unchecked future-discovery block option. |
| **Dither** | [`Dither.tsx`](Dither.tsx) | SVG/Canvas brutalist dithering pattern component used as drop zone backgrounds and visual texture accents. |
| **EmptyState** | [`EmptyState.tsx`](EmptyState.tsx) | High-contrast placeholder for zero-result states, missing search hits, or empty pipeline stages. Includes title, description, and optional action button. |
| **FavoriteButton** | [`FavoriteButton.tsx`](FavoriteButton.tsx) | Star icon toggle button providing instant optimistic toggling of a job's `favorited` flag. |
| **Field** | [`Field.tsx`](Field.tsx) | Form field wrapper binding labels, required asterisks, helper text, and error validation messages to form controls. |
| **Input** | [`Input.tsx`](Input.tsx) | Monospaced text input with 0px radius, 1px border, clear button, and optional prefix/suffix icons. |
| **KeyHint** | [`KeyHint.tsx`](KeyHint.tsx) | Monospaced keyboard shortcut pill (e.g. `⌘K`, `J`, `K`, `ESC`) used throughout buttons, menus, and footer bars. |
| **ListRow** | [`ListRow.tsx`](ListRow.tsx) | Standardized hoverable list item container with left/right slots and border separators. |
| **Modal** | [`Modal.tsx`](Modal.tsx) | Accessible dialog backdrop and container. Manages focus trapping, `Escape` key listeners, and backdrop click dismissal. |
| **Panel** | [`Panel.tsx`](Panel.tsx) | Heavy-bordered content container with optional title header bar and collapsible body. |
| **RangeSlider** | [`RangeSlider.tsx`](RangeSlider.tsx) | Dual-thumb brutalist range slider for bounded numeric ranges (salary minimum/maximum, years of experience). Supports step increments and thumb tooltips. |
| **Rule** | [`Rule.tsx`](Rule.tsx) | Hairline 1px horizontal or vertical divider (`hr` / `div`) styled with `--color-border`. |
| **Select** | [`Select.tsx`](Select.tsx) | Native select with shared control height, global font inheritance, theme colors, and platform keyboard behavior. |
| **Table** | [`Table.tsx`](Table.tsx) | Monospaced data table components (`Table`, `TableHead`, `TableRow`, `TableCell`) with sort headers and zebra striping. |
| **Tag** | [`Tag.tsx`](Tag.tsx) | Pill-style badge tag displaying role categories, workplace arrangements (`IN-OFFICE`, `REMOTE`, `HYBRID`), and status labels. |
| **Toast** | [`Toast.tsx`](Toast.tsx) | Floating alert notification item displaying progress countdown, status icon, and dismiss button. |
| **Toggle** | [`Toggle.tsx`](Toggle.tsx) | Binary toggle switch with brutalist square indicator and custom checked styling. |
| **Tooltip** | [`Tooltip.tsx`](Tooltip.tsx) | Instant hover tooltip computing geometric coordinates via `src/lib/positioning.ts` to prevent window edge overflow. |

Job removal uses a compact dialog with an initially unchecked future-discovery
block option. Native dialog focus begins on Cancel; Enter activates the focused
control and Escape restores focus. All saved copies of the posting are removed
softly, and the success toast offers Undo. Settings → Removed & blocked jobs
provides paginated search, restoration, blocking and unblocking.

Control dimensions live in `src/styles/index.css`: 32/36/40px small/default/large
heights, shared spacing, and a 32px minimum icon target. SVG sizes never determine
the clickable area. Use contextual button names; icons remain `aria-hidden`.

The app theme explicitly sets native `color-scheme` to light or dark. Scrollbars
use semantic thumb/track/hover tokens with standard properties and a WebKit
fallback for older WKWebView versions. Vertical scrolling containers request a
stable gutter where supported; actual tables retain horizontal scrolling. No
JavaScript scrollbar or separate icon font is used.
