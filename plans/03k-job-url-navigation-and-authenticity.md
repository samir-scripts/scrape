# Sub-plan 03k: Job URL Navigation & Authenticity Engine

> Architectural plan for ensuring all job URLs in Scrape navigate reliably to real, active job application pages, providing clickable interactive links in the UI with a resilient browser fallback, and ensuring mock/seed workflows never emit false or dead URLs.

---

## 1. Context & Problem Statement

Users identified two critical defects in job URL handling:
1. **Clicking the URL does not navigate to the job**: In the consolidated job window, the URL string was rendered in an inert plain-text `<span>` rather than a clickable `<a>` anchor. Furthermore, `handleOpenUrl` invoked `@tauri-apps/plugin-opener` directly without a browser fallback (`window.open`), causing silent failures in standard web environments.
2. **Entering the URL in a browser redirected to the company root homepage (`cloudflare.com/careers/`)**: In mock and development data, `api.ts` generated synthetic dummy URLs with IDs like `/jobs/1000`. Because Greenhouse job ID `1000` is non-existent, Greenhouse issued an HTTP 302 redirect to Cloudflare's generic corporate marketing page instead of an application page, preventing the candidate from applying.

---

## 2. Technical Design & Architecture

### 2.1 Interactive URL Anchor in UI
- In `JobDetail.tsx`, `JobMetaPanel.tsx`, and `JobDetailDrawer.tsx`:
  - Wrap `{job.url}` in `<a href={job.url} target="_blank" rel="noopener noreferrer">` with brutalist styling (underline on hover, high contrast, external icon `[↗]`).
  - Attach `onClick={(e) => { e.preventDefault(); onOpenUrl(job.url); }}` so the centralized opener handles the event while native middle-click / context menu retains native browser anchor semantics.
- In `JobCard.tsx`:
  - Add an external open button `[↗]` on the card header so candidates can open a posting directly without needing to open the side pane first.

### 2.2 Resilient Native + Web Opener Pipeline
- In `src/App.tsx`:
  - If running inside Tauri (`isTauri()` is true), call `openUrl(url)` from `@tauri-apps/plugin-opener`.
  - If `openUrl` throws or if `isTauri()` is false, fall back immediately to `window.open(url, "_blank", "noopener,noreferrer")`.
  - Ensure zero silent failures.

### 2.3 Authentic Job IDs, 1-to-1 Title Mapping & Policy Filtering
- In `src/lib/api.ts`:
  - Enforce 1-to-1 authentic titles that match the destination Greenhouse ATS page:
    - `8199958`: `"Software Engineer Intern (2027) - Austin, TX"`
    - `8245197`: `"Software Engineer Intern (2027)"`
    - `8245211`: `"Software Engineer Intern (2027)"`
    - `8187404`: `"Systems Engineer, Network Protocols & Distributed Systems"`
    - `8247492`: `"Technical Product Marketing Manager"`
    - `7695702`: `"Account Executive, FedCiv"`
    - `8097321`: `"AI Security Research & Red Team Engineer"`
    - `7742773`: `"Systems Engineer, MAPS"`
  - Construct direct URLs: `https://job-boards.greenhouse.io/{board}/jobs/{id}?gh_jid={id}`.
  - Implement full candidate pool policy filtering inside `generateScrapedJobsForSource` honoring `role_types`, `exclude_keywords` (e.g. `"Marketing"`), and `include_keywords`.
  - Guarantees that postings matching exclusion criteria (such as `"Technical Product Marketing Manager"` under `exclude_keywords: ["MBA", "Sales", "Marketing"]`) are rejected, and any admitted role's title 100% matches its actual destination ATS page.
- In `crates/scrape-worker/src/adapters/greenhouse.rs`:
  - Modernize legacy `boards.greenhouse.io` URLs to `job-boards.greenhouse.io` during ingestion, eliminating the 301 redirect hop.

---

## 3. Implementation Steps

1. **`src/App.tsx`**: Update `handleOpenUrl` with `window.open` fallback.
2. **`src/components/views/discovered/JobDetail.tsx`**: Replace inert `<span>` with interactive `<a>` anchor link.
3. **`src/components/views/discovered/JobCard.tsx`**: Add external link button `[↗]` on job cards.
4. **`src/components/views/discovered/JobMetaPanel.tsx`** & **`src/components/views/tracked/JobDetailDrawer.tsx`**: Update URL display to clickable anchors.
5. **`src/lib/api.ts`**: Update `generateScrapedJobsForSource` with active Cloudflare job IDs and `job-boards.greenhouse.io` URLs.
6. **`crates/scrape-worker/src/adapters/greenhouse.rs`**: Modernize URLs to `job-boards.greenhouse.io`.
7. **Automated Tests**:
   - Update `src/test/discovered_card_view_and_consolidated_window.test.ts`.
   - Add `src/test/job_url_navigation_and_authenticity.test.ts`.
   - Verify with `npx tsc --noEmit`, `npm test`, `cargo test --workspace`, and `cargo clippy`.

---

## 4. Verification & Acceptance Criteria
- [x] Clicking the job URL text or the `[OPEN POSTING]` button opens the job page in both Tauri and browser environments.
- [x] URL is rendered as an interactive anchor link (`<a href="..." target="_blank">`).
- [x] JobCard provides direct external open action.
- [x] Mock/seed job URLs point to real, live Greenhouse application forms and never redirect to root `cloudflare.com/careers/`.
- [x] All automated tests pass cleanly with zero Clippy warnings or TypeScript errors.
