# Phase 3.14: Discovered Card View & Consolidated Job Window Specification

> **Super-plans:** [`00-master-plan.md`](./00-master-plan.md) · [`03-frontend-ui.md`](./03-frontend-ui.md)  
> **Status:** SPECIFICATION COMPLETE & READY FOR EXECUTION  
> **Related Decisions:** Locked Decision 50

---

## 1. Problem Statements & Context

1. **Information Fragmentation across 3 Panes:**
   - Previously, Discovered inbox used a 3-pane layout: Left (List), Center (JobDetail), Right (JobMetaPanel).
   - Splitting metadata (salary, clearance, dates, sponsorship, triage buttons) into a separate 3rd panel created visual clutter and cognitive overhead on standard laptop screens.
2. **Missing Card View Default:**
   - Users expect a clean, readable, vertically stacked "card-like" stream of jobs with clear typography, company logos, dates, and badges (matching user screenshot reference).
   - When no job is clicked, the app should display the card stream in a clean 1-pane view rather than forcing an empty or arbitrary job detail pane on startup.
3. **Dynamic 1-Pane to 2-Pane Transition:**
   - Clicking a job card should open the side window, dynamically putting the user in a 2-pane view (Left: Card stream, Right: Consolidated Job Window).
   - Closing the side window (`[✕]` or `Esc`) should return the user to the full 1-pane card view.
4. **Consolidated Job Window:**
   - All status, compensation, eligibility, evidence quotes, triage actions, description, and notes must live within the single job window.

---

## 2. Visual Architecture & Layout States

### 2.1 State 1: 1-Pane Card View (`selectedJobId === null`)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ TOPNAV: DISCOVERED (42)   TRACKED   METRICS   SCRAPER   SETTINGS             │
├──────────────────────────────────────────────────────────────────────────────┤
│ SUBNAV: [FILTERS ▾]                                       42 POSTINGS FOUND  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ──────────────────────────────────────────────────────────────────────────  │
│  New Grad: Software Engineer                                            [★]  │
│  Quora                                                          Aug 5, 2026  │
│  [ Remote ]  [ $140K - $180K ]  [ New Grad ]  [ Full-Time ]                  │
│  ──────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  ──────────────────────────────────────────────────────────────────────────  │
│  Systems Engineer - Edge Infrastructure                                 [★]  │
│  Cloudflare                                                     Aug 4, 2026  │
│  [ Hybrid ]  [ Austin, TX ]  [ $150K - $190K ]  [ Full-Time ]                │
│  ──────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  ──────────────────────────────────────────────────────────────────────────  │
│  Product Designer                                                       [☆]  │
│  Figma                                                          Aug 2, 2026  │
│  [ Remote ]  [ $130K - $160K ]  [ Visa: Yes ]  [ Full-Time ]                 │
│  ──────────────────────────────────────────────────────────────────────────  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

- Clean centered/comfortable max-width stream (`max-w-4xl mx-auto w-full`).
- Each card has padding (`p-4` or `py-3.5 px-4`), 1px dividers, hover highlight, company logo, favorite button, formatted date (`Aug 5, 2026`), and status badges.

---

### 2.2 State 2: 2-Pane View (`selectedJobId !== null`)

```
┌──────────────────────────────────────┬───────────────────────────────────────┐
│ DISCOVERED CARDS (42)                │ JOB DETAILS ▏ [ESC]        [✕] [↵] [★]│
├──────────────────────────────────────┼───────────────────────────────────────┤
│ New Grad: Software Engineer      [★] │ [A] APPLY   [X] IGNORE   [V] VIEWED   │
│ Quora                    Aug 5, 2026 │ ───────────────────────────────────── │
│ [ Remote ] [ $140K-$180K ]           │ NEW GRAD: SOFTWARE ENGINEER           │
│ ──────────────────────────────────── │ Quora · Remote · Full-Time            │
│ Systems Engineer                 [★] │ ───────────────────────────────────── │
│ Cloudflare               Aug 4, 2026 │ COMPENSATION & WORKPLACE              │
│ [ Hybrid ] [ $150K-$190K ]           │ Salary: $140K - $180K  Workplace: Remote│
│ ──────────────────────────────────── │ ───────────────────────────────────── │
│ Product Designer                 [☆] │ REQUIREMENTS & CLEARANCE              │
│ Figma                    Aug 2, 2026 │ Exp: 0-1 Yrs  Grad: Yes  Visa: Sponsors│
│ [ Remote ] [ $130K-$160K ]           │ ───────────────────────────────────── │
│                                      │ JOB DESCRIPTION ▏          1,840 CHARS│
│                                      │ Build high-impact services for...     │
│                                      │ ───────────────────────────────────── │
│                                      │ CANDIDATE NOTES ▏                     │
│                                      │ [ Autosaved notes textarea...       ] │
└──────────────────────────────────────┴───────────────────────────────────────┘
```

- **Left Pane:** Resizable card stream (default 440px, min 320px, max 700px). The active card has a high-contrast accent indicator.
- **Divider:** 1px interactive resizable splitter with pointer capture, double-click reset to 440px, and `localStorage` layout persistence.
- **Right Pane (Consolidated Job Window):** Comprehensive detail viewer containing header controls, action strip, metadata boxes, description viewer, and notes editor.

---

## 3. Component Architecture & Implementation Details

### 3.1 `JobCard.tsx` (`src/components/views/discovered/JobCard.tsx`)
- Specialized brutalist card component rendering:
  - Header line: Bold title + FavoriteButton + Status tag.
  - Sub-line: Company logo + Company name on the left, formatted date (`toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })`) on the right.
  - Badges row:
    - Workplace tag (emerald styling for `Remote` matching user screenshot).
    - Salary badge (`formatSalary(job)`).
    - Career level / New Grad badge (`is_new_grad === "yes"`).
    - Role type badge (`Full-Time`, `Internship`).
    - Visa sponsorship / Clearance badges.
  - Description snippet preview (2-line truncated).
  - Selected state: `border-l-4 border-l-[var(--color-lime)] bg-[var(--color-paper-2)]/80` or active brutalist styling.
  - Hover state: `hover:bg-[var(--color-paper-2)]/50 cursor-pointer`.

### 3.2 `DiscoveredTwoColumn.tsx` (`src/components/layout/DiscoveredTwoColumn.tsx`)
- Dynamic 1-to-2 pane layout container:
  - If `selectedJob` is null: Renders left child at 100% width with no dividers or empty panes.
  - If `selectedJob` is non-null: Renders left child at `leftWidth` (resizable between 320px and 700px), 1px pointer-captured vertical splitter, and right child filling `flex-1`.
  - Persists width to `localStorage` under `scrape:layout:discovered_split_width`.
  - Double-click reset to default `440px`.

### 3.3 `JobDetail.tsx` Consolidation (`src/components/views/discovered/JobDetail.tsx`)
- Merges all sections from `JobMetaPanel.tsx`:
  - **Top Navigation Bar**: Window title, Status badge, Role tag, FavoriteButton, External Link (`[↵] OPEN`), Close Button (`[✕ CLOSE] [ESC]`).
  - **Triage Action Strip**: `[A] MARK AS APPLIED`, `[X] IGNORE`, `[V] MARK VIEWED`.
  - **Main Heading**: Large bold title, Company logo avatar, Company name, Location, Workplace type.
  - **Compensation & Geography Box**: Salary, salary interval, workplace type, parsed locations.
  - **Requirements & Clearance Box**: Experience requirements, New Grad status, Visa sponsorship policy, Clearance level, evidence quote callouts.
  - **Posting Telemetry**: Date posted, Date added, Last seen.
  - **Job Description Viewer**: Formatted plain text, char count, scrollable bounds.
  - **Candidate Notes Editor**: Debounced autosaving textarea.

### 3.4 `DiscoveredView.tsx` Refactoring
- Default view is the card stack view.
- Sets initial `selectedJobId` to `null` (rather than forcing first job selection).
- When a card is clicked, sets `selectedJobId = job.id` to enter 2-pane view.
- Escape hotkey or close button sets `selectedJobId = null` to return to 1-pane card view.
- `J` / `K` keyboard navigation steps through cards in both 1-pane and 2-pane modes.

### 3.5 `SubNav.tsx` Cleanup
- For `view === "discovered"`:
  - Remove `[■] 3-PANE` and `[≡] TABLE` toggles (card view is the default and only view).
  - Display clean postings count (`42 POSTINGS FOUND`) and close button `[✕] CLOSE WINDOW [ESC]` when a job is currently selected.

---

## 4. Verification & Testing Matrix

1. **Vitest Unit Tests (`src/test/discovered_card_view_and_consolidated_window.test.ts`)**:
   - `JobCard`: renders title, company, formatted date (`Aug 5, 2026`), remote badge, salary badge, favorite button.
   - `DiscoveredTwoColumn`: renders 1-pane layout when right child is null, renders 2-pane layout with resizable separator when right child is present.
   - `JobDetail`: verifies all consolidated metadata fields (compensation, requirements, evidence quotes, triage actions, close button).
   - `DiscoveredView`: verifies initial 1-pane state (`selectedJobId === null`), selection transition to 2-pane, and close transition back to 1-pane.
2. **Regression Verification**:
   - `npx tsc --noEmit`
   - `npm test`
   - `cargo test --workspace`
   - `cargo clippy --all-targets -- -D warnings`
   - `npm run build`
