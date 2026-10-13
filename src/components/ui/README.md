# `src/components/ui` — Reusable Brutalist UI Primitives

## Overview

The `src/components/ui` directory contains atomic, reusable UI components implementing Scrape's NieR:Automata brutalist design system. Each primitive enforces strict geometric constraints (0px border radiuses, 1px rules, uppercase typography, and monospaced accents).

---

## Component Manifest

| Component | File | Responsibilities & Props |
|---|---|---|
| **Button** | [`Button.tsx`](Button.tsx) | Brutalist button supporting variants (`primary`, `secondary`, `ghost`, `danger`), sizes (`sm`, `md`, `lg`), high-contrast inverted active states, and keyboard focus outlines. |
| **ClockPicker** | [`ClockPicker.tsx`](ClockPicker.tsx) | Interactive 24-hour circular analog clock face component. Allows users to click or drag radial hands to pick crawl execution hours and minutes, updating cron expressions. |
| **CompanyLogo** | [`CompanyLogo.tsx`](CompanyLogo.tsx) | Company favicon image renderer using Google favicon proxy URLs with automatic fallback to a monospaced initial box on network errors or missing logos. |
| **DeleteJobModal** | [`DeleteJobModal.tsx`](DeleteJobModal.tsx) | Confirmation modal for deleting jobs. Displays the target job title, warns the user that deletion permanently tombstones the posting from future crawls, and includes a "Don't ask again" preference checkbox. |
| **Dither** | [`Dither.tsx`](Dither.tsx) | SVG/Canvas brutalist dithering pattern component used as drop zone backgrounds and visual texture accents. |
| **EmptyState** | [`EmptyState.tsx`](EmptyState.tsx) | High-contrast placeholder for zero-result states, missing search hits, or empty pipeline stages. Includes title, description, and optional action button. |
| **FavoriteButton** | [`FavoriteButton.tsx`](FavoriteButton.tsx) | Star icon toggle button providing instant optimistic toggling of a job's `is_favorite` flag. |
| **Field** | [`Field.tsx`](Field.tsx) | Form field wrapper binding labels, required asterisks, helper text, and error validation messages to form controls. |
| **Input** | [`Input.tsx`](Input.tsx) | Monospaced text input with 0px radius, 1px border, clear button, and optional prefix/suffix icons. |
| **KeyHint** | [`KeyHint.tsx`](KeyHint.tsx) | Monospaced keyboard shortcut pill (e.g. `⌘K`, `J`, `K`, `ESC`) used throughout buttons, menus, and footer bars. |
| **ListRow** | [`ListRow.tsx`](ListRow.tsx) | Standardized hoverable list item container with left/right slots and border separators. |
| **Modal** | [`Modal.tsx`](Modal.tsx) | Accessible dialog backdrop and container. Manages focus trapping, `Escape` key listeners, and backdrop click dismissal. |
| **Panel** | [`Panel.tsx`](Panel.tsx) | Heavy-bordered content container with optional title header bar and collapsible body. |
| **RangeSlider** | [`RangeSlider.tsx`](RangeSlider.tsx) | Dual-thumb brutalist range slider for bounded numeric ranges (salary minimum/maximum, years of experience). Supports step increments and thumb tooltips. |
| **Rule** | [`Rule.tsx`](Rule.tsx) | Hairline 1px horizontal or vertical divider (`hr` / `div`) styled with `--color-border`. |
| **Select** | [`Select.tsx`](Select.tsx) | Custom brutalist select dropdown replacing native browser selects with keyboard-navigable options. |
| **Table** | [`Table.tsx`](Table.tsx) | Monospaced data table components (`Table`, `TableHead`, `TableRow`, `TableCell`) with sort headers and zebra striping. |
| **Tag** | [`Tag.tsx`](Tag.tsx) | Pill-style badge tag displaying role categories, workplace arrangements (`IN-OFFICE`, `REMOTE`, `HYBRID`), and status labels. |
| **Toast** | [`Toast.tsx`](Toast.tsx) | Floating alert notification item displaying progress countdown, status icon, and dismiss button. |
| **Toggle** | [`Toggle.tsx`](Toggle.tsx) | Binary toggle switch with brutalist square indicator and custom checked styling. |
| **Tooltip** | [`Tooltip.tsx`](Tooltip.tsx) | Instant hover tooltip computing geometric coordinates via `src/lib/positioning.ts` to prevent window edge overflow. |
