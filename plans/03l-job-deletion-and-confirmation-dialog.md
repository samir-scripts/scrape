# Sub-plan 03l: Job Deletion & Confirmation Dialog Engine

> Architectural plan for permanent job deletion across Discovered and Tracked views, featuring a brutalist confirmation modal ("Are you sure you want to delete?") with a "Don't ask again" persistent checkbox, settings integration to toggle confirmation behavior, and a SQLite tombstone engine preventing scraper resurrection.

---

## 1. Context & User Requirements

Users need the ability to delete unwanted or obsolete jobs directly from both **DISCOVERED** and **TRACKED** views.

### Specific User Requirements:
1. **Deletion Trigger**: Jobs can be deleted from:
   - **Discovered**:
     - Consolidated Job Window (`JobDetail.tsx`) via Triage toolbar action `[DEL] DELETE`.
     - Streamlined Card View (`JobCard.tsx`) via quick action `[DEL]`.
     - Table View (`DiscoveredTable.tsx`) via row triage action `DEL`.
     - Keyboard shortcut: `[Delete]` or `[D]`.
   - **Tracked**:
     - Detail Drawer (`JobDetailDrawer.tsx`) via danger action `[DEL] DELETE JOB`.
     - Kanban Board (`KanbanColumn.tsx`) via card action / context trigger.
     - Tracked List (`TrackedList.tsx`) via row action `DEL`.
     - Keyboard shortcut: `[Delete]` or `[D]`.
2. **Confirmation Modal**:
   - Triggers when user requests deletion and confirmation is enabled.
   - Header: `CONFIRM DELETION // DANGER`
   - Prompt: `"Are you sure you want to delete?"`
   - Context: Displays Job Title, Company, and irreversible warning.
   - Checkbox: `"Don't ask again"` (unselected by default).
   - Actions: `[DEL] DELETE [↵]` (danger highlight) and `[ESC] CANCEL`.
   - If `"Don't ask again"` is checked upon confirming, the setting `app_confirm_job_deletion` is set to `false`.
3. **Settings Integration ("Can be untoggled in settings")**:
   - In **Settings -> System**, provide a toggle:
     `CONFIRM BEFORE DELETING JOBS`
     Description: `Prompt for confirmation before deleting jobs from Discovered or Tracked`.
   - When `"Don't ask again"` was previously checked in the modal, this toggle reflects `OFF` (`false`).
   - The user can switch it back `ON` (`true`) at any time to restore deletion confirmation prompts.
4. **Resurrection Prevention (The Scraper Re-Ingestion Problem)**:
   - When a job is deleted from SQLite `jobs`, future scheduled or manual scrape runs would re-discover the same job posting and insert it back into `jobs`.
   - To make deletion permanent, a tombstone table `deleted_jobs` tracks `dedupe_key`. Scraper upsert checks this tombstone and skips deleted postings.

---

## 2. Technical Architecture & Component Design

### 2.1 Database Schema & Tombstone Engine
- **Migration `0010_deleted_jobs_tombstone.sql`**:
  ```sql
  -- Migration 0010: Tombstone table for deleted jobs
  CREATE TABLE IF NOT EXISTS deleted_jobs (
    dedupe_key TEXT PRIMARY KEY,
    deleted_at TEXT NOT NULL
  );
  CREATE INDEX IF NOT EXISTS idx_deleted_jobs_time ON deleted_jobs(deleted_at);
  ```
- **Cascade & Cleanup**:
  - `jobs` foreign key constraints in `job_status_events` and `job_locations` already use `ON DELETE CASCADE`.
  - When `JobsRepo::delete(&pool, id)` executes:
    1. Reads `dedupe_key` for `id`.
    2. Inserts `dedupe_key` into `deleted_jobs`.
    3. Executes `DELETE FROM jobs WHERE id = ?1`.
  - In `JobsRepo::upsert_scraped`:
    - Checks if `job.dedupe_key` exists in `deleted_jobs`.
    - If found, skips insertion and increments `stats.skipped`.

### 2.2 Settings Model & Configuration
- **Backend `src-tauri/src/db/settings.rs`**:
  - Add `pub app_confirm_job_deletion: bool` (default `true`) to `Settings` struct.
  - Constant: `const KEY_APP_CONFIRM_JOB_DELETION: &'static str = "app.confirm_job_deletion";`.
  - Register in `ensure_defaults`, `get_all`, and `set_value`.
  - Generate TypeScript bindings `src/bindings/Settings.ts` via `ts-rs`.
- **Tauri Commands**:
  - `delete_job(app: AppHandle, state: State<AppState>, id: String) -> Result<bool>`
    - Deletes job, records tombstone, emits `jobs://changed`.
- **Frontend Settings UI (`SystemSettings.tsx`)**:
  - Add Toggle:
    ```tsx
    <Toggle
      checked={settings.app_confirm_job_deletion}
      onChange={(checked) =>
        onUpdateSetting("app.confirm_job_deletion", checked)
      }
      label="CONFIRM BEFORE DELETING JOBS"
      description="PROMPT FOR CONFIRMATION BEFORE DELETING JOBS FROM DISCOVERED OR TRACKED"
    />
    ```

### 2.3 Modal Component: `DeleteJobModal.tsx`
- Strict PS2-era / NieR brutalist aesthetic (0px border-radius, 1px/2px rules, high contrast):
  - Uses native `<dialog>` wrapper with backdrop dither.
  - Header: `DELETE JOB POSTING`
  - Prompt: `"Are you sure you want to delete this job posting?"`
  - Target summary:
    - Company name & job title
    - Current stage / status
    - Notice: `"This action permanently removes the job and prevents future re-scraping."`
  - Brutalist Checkbox:
    - `[X] DON'T ASK AGAIN` (can be toggled by Space or Click).
  - Button actions:
    - Primary: `[DEL] DELETE [↵]` (danger styling)
    - Cancel: `[ESC] CANCEL`

### 2.4 State Management & Trigger Handlers
- In `App.tsx`:
  - `deleteJobMutation` with TanStack optimistic cache removal and invalidation of `["jobs"]` and `["metrics"]`.
  - Central modal state or scoped modal handlers:
    - State: `jobPendingDelete: Job | null`
    - Handler `requestDeleteJob(job: Job)`:
      - If `!settings.app_confirm_job_deletion`:
        - Immediately invoke `deleteJobMutation.mutate(job.id)`.
      - If `settings.app_confirm_job_deletion`:
        - Set `jobPendingDelete = job` to render `DeleteJobModal`.
    - Confirm handler `confirmDeleteJob(dontAskAgain: boolean)`:
      - If `dontAskAgain`, call `updateSettingMutation.mutate({ key: "app.confirm_job_deletion", value: false })`.
      - Call `deleteJobMutation.mutate(jobPendingDelete.id)`.
      - Clear `jobPendingDelete`.

### 2.5 View & Hotkey Integrations
- **Discovered View**:
  - `JobDetail.tsx`: Add `[DEL] DELETE` in Triage toolbar.
  - `JobCard.tsx`: Add `[DEL]` button.
  - `DiscoveredTable.tsx`: Add `DEL` in row triage column.
  - Hotkey: `Delete` / `D` triggers deletion for selected job.
- **Tracked View**:
  - `JobDetailDrawer.tsx`: Add `[DEL] DELETE APPLICATION` button.
  - `TrackedList.tsx`: Add `DEL` in Actions column.
  - `KanbanColumn.tsx`: Card delete button.
  - Hotkey: `Delete` / `D` triggers deletion for selected job.

---

## 3. Implementation Steps

1. **Database Migration**:
   - Create `src-tauri/migrations/0010_deleted_jobs_tombstone.sql`.
2. **Backend Repos & Commands**:
   - Update `src-tauri/src/db/settings.rs` with `app_confirm_job_deletion`.
   - Update `src-tauri/src/db/jobs.rs`:
     - Add `JobsRepo::delete(&pool, id)`.
     - Update `JobsRepo::upsert_scraped` to check `deleted_jobs`.
   - Update `src-tauri/src/commands/jobs.rs`:
     - Add `delete_job` command and register in `src-tauri/src/lib.rs`.
3. **Frontend API & Store**:
   - Update `src/lib/api.ts` with `deleteJob(id: string): Promise<boolean>` (including mock data support in web dev).
   - Update `src/bindings/Settings.ts` via `cargo test` / `ts-rs`.
4. **UI Components**:
   - Create `src/components/ui/DeleteJobModal.tsx`.
   - Update `SystemSettings.tsx` to render the confirmation toggle.
   - Update `DiscoveredView.tsx`, `JobDetail.tsx`, `JobCard.tsx`, `DiscoveredTable.tsx`.
   - Update `TrackedView.tsx`, `JobDetailDrawer.tsx`, `TrackedList.tsx`, `KanbanColumn.tsx`.
   - Update `useHotkeys.ts` with `onDelete` handler.
   - Wire delete mutation and modal in `App.tsx`.
5. **Testing**:
   - Rust tests for `delete`, `deleted_jobs` tombstone deduplication, and settings toggle.
   - Vitest test suite `src/test/job_deletion_and_confirmation.test.ts`.

---

## 4. Verification & Acceptance Criteria

- [x] Deletion is accessible from Discovered (cards, detail window, table) and Tracked (drawer, list, kanban).
- [x] When confirmation is enabled (default), clicking delete presents modal `"Are you sure you want to delete?"`.
- [x] Checking `"Don't ask again"` and confirming deletes the job and sets `app.confirm_job_deletion` to `false`.
- [x] When `app.confirm_job_deletion` is `false`, future deletions proceed immediately without modal.
- [x] Toggling `CONFIRM BEFORE DELETING JOBS` in Settings re-enables the confirmation modal.
- [x] Deleted jobs are recorded in `deleted_jobs` tombstone table and never re-scraped or resurrected.
- [x] All automated Rust tests and Vitest suites pass cleanly with zero Clippy warnings.
