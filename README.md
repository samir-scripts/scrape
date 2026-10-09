# SCRAPE

Local-first job scraper and application tracker. Lives in the system tray, scrapes job boards on a schedule, and tracks your applications in a brutalist, NieR:Automata-inspired interface.

**Stack:** Tauri v2 · Rust · SQLite (sqlx) · React + TypeScript · Tailwind CSS v4 · Visx

## Plans

See [`plans/00-master-plan.md`](plans/00-master-plan.md) for the architecture and phase-by-phase implementation plans.

## Branching

| Branch | Purpose |
|---|---|
| `main` | Stable, released code. Tags `vX.Y.Z` trigger release builds. |
| `develop` | Integration branch. Default PR target. |
| `feature/*`, `fix/*`, `chore/*`, `docs/*` | Short-lived work branches → PR into `develop`. |
| `hotfix/*` | Urgent fixes → PR into `main`, then back-merge to `develop`. |

Details: [Master plan §7](plans/00-master-plan.md#7-git-branching--github-actions).

## Development

Prerequisites: Rust (stable), Node.js LTS, and the [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev
```
