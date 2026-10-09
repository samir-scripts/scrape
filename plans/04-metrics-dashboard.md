# Phase 4 — Data Visualization & Metrics Dashboard

**Goal:** Show application metrics in a flat, monochrome, dithered style that belongs in the same UI.

← [Phase 3](./03-frontend-ui.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 5](./05-ci-cd.md)

---

## 4.1 Library decision

| Library | Rendering | Styling control | Verdict |
|---|---|---|---|
| **Visx** | SVG primitives | Total — we write every `<rect>`/`<path>`; no default tooltips, gradients, or radii to strip | ✅ **Selected** |
| Recharts | SVG components | Opinionated defaults (animations, tooltips, rounded bars) need overriding per component | Fallback |
| Chart.js | Canvas | Anti-aliasing fights crisp 1px/pixelated look; patterns harder; not inspectable | ❌ |

Packages: `@visx/scale`, `@visx/shape`, `@visx/axis`, `@visx/group`, `@visx/grid`, `@visx/pattern`, `@visx/responsive`. **No** `@visx/tooltip` — see readout below.

**Chart style rules**
- `shape-rendering="crispEdges"` on bars/axes/grid; integer-snapped coordinates.
- Fills only from palette or `@visx/pattern` (`PatternCircles` = halftone, `PatternLines` = hatch, custom 2px checker) — legible in monochrome.
- Axes: 1px ink, uppercase 10px labels, ticks outward, no gridline except a dotted baseline.
- No animation by default; no hover tooltips. Instead, hovering/arrow-keying a bar updates a fixed **READOUT** panel (NieR status-panel style): `WEEK 41 · 2026-10-05 → 10-11 · 7 APPLIED`.

## 4.2 Backend aggregation (`src-tauri/src/db/metrics.rs`)

Command `get_metrics({ granularity: 'week' | 'month', periods: number }) -> Metrics`:

```ts
type Metrics = {
  totals: {
    scraped: number;
    applied: number;
    interviewing: number;
    offers: number;
    response_rate: number;
    median_salary: number | null;
    avg_salary: number | null;
  };
  series: { bucket_start: string; label: string; count: number }[];     // zero-filled, oldest → newest
  by_role_type: { role_type: RoleType; count: number }[];               // jobs ever applied, by type
  salary_distribution: { range_label: string; count: number }[];        // e.g. "<$100k", "$100k-$150k", "$150k-$200k", ">$200k"
  streak: { current: number; longest: number; last_applied_on: string | null };
};
```

Definitions:
- **Applied date** = first `job_status_events.changed_at` where `to_status = 'applied'` (one per job).
- **Total applied** = jobs that have ever reached `applied` (includes later stages).
- **Series** buckets computed in Rust with `chrono-tz` using `system.timezone`; weeks start on `system.week_start`.
- **Current streak** = consecutive local calendar days with ≥1 application, ending today — or yesterday (so the streak isn't "broken" before you've applied today).
- **Response rate** = jobs reaching `interviewing|offer` ÷ total applied.
- **Salary analytics** = computed over jobs with valid `salary_min` or `salary_max`.

- [ ] Unit tests: zero-fill, week-start Sunday vs Monday, DST transition week, streak edge cases (today/yesterday/gap), multiple status flips counted once.

## 4.3 Dashboard layout (Metrics view)

```
┌ TOTAL SCRAPED ┐┌ TOTAL APPLIED ┐┌ RESPONSE RATE ┐┌ MEDIAN SALARY ┐
│        1,284  ││          86   ││       12%     ││     $165,000  │
│ ░░ +63 / 7D   ││ ▓ +7 / 7D     ││ 10 / 86       ││ $120K - $220K │
└───────────────┘└───────────────┘└───────────────┘└───────────────┘
┌ APPLICATIONS OVER TIME     RANGE: [ 12 WEEKS ▾ ] ┐┌ JOB TYPES APPLIED ─┐
│  ▇                                               ││     ◯ donut        │
│  ▇ ▇     ▇                                       ││  ■ FULL-TIME  62%  │
│  ▇ ▇ ▇ ▇ ▇ ▇ ...                                 ││  ▒ CONTRACT   20%  │
│ ──────────────────────────────                   ││  ░ INTERN     14%  │
│ READOUT: WEEK 41 · 7 APPLIED                     ││  ▦ UNKNOWN     4%  │
└──────────────────────────────────────────────────┘└────────────────────┘
```

- [ ] Header chrome is direct and utilitarian; zero vanity copy (no "COMPUTED IN REAL TIME", "PHASE 4 ENGINE PREVIEW").
- [ ] CSS grid: 4 metric cards row; chart row `2fr 1fr`; collapses to single column under 1100px.
- [ ] Granularity and time-range filter via dropdown (`RANGE: [ 12 WEEKS ▾ ]`).
- [ ] **MetricCard:** label (uppercase, ink-mute), big tabular-nums value, sub-line delta; halftone corner decoration.
- [ ] **ApplicationsChart** (`@visx/shape` `Bar` + `scaleBand`/`scaleLinear`): last 12 weeks or 12 months; selected bar = solid ink, others = `PatternCircles` halftone; `WEEK/MONTH` toggle as inverted-block segmented control; keyboard `←/→` to move selection.
- [ ] **RoleTypeDonut** (`@visx/shape` `Pie`, `padAngle=0`, 1px ink stroke between slices): each slice a distinct pattern (solid · hatch · dots · checker · paper); legend table with counts + %, hover/arrow selection syncs with readout.
- [ ] Empty state: dithered placeholder + `NO APPLICATIONS LOGGED. PRESS [A] ON A JOB TO BEGIN.`
- [ ] Refetch on `jobs://changed`.

## Acceptance Criteria
- [ ] With seeded data (`npm run seed` → dev-only command inserting ~200 jobs/events), all charts and cards render correct values matching SQL spot-checks.
- [ ] Marking a job `applied` updates cards and the current-week bar without reload.
- [ ] Charts are crisp at 1× and 2× DPI; no rounded corners, gradients, shadows, or floating tooltips.
- [ ] Both themes (paper/obsidian) render legibly.
