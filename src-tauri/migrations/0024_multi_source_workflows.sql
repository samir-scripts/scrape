-- Migration 0024: Multi-source workflow ownership and fair budgeting

CREATE TABLE IF NOT EXISTS workflows (
  id                    TEXT PRIMARY KEY,
  name                  TEXT NOT NULL,
  enabled               INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  cadence               TEXT NOT NULL DEFAULT 'daily'
                        CHECK (cadence IN ('global','hourly','every_6h','every_12h','daily','weekly','manual','cron')),
  cadence_cron          TEXT,
  next_run_at           TEXT,
  retention_days        INTEGER NOT NULL DEFAULT 30 CHECK (retention_days >= 0),
  retention_policy      TEXT NOT NULL DEFAULT 'untracked_only' CHECK (retention_policy IN ('untracked_only', 'all')),
  filters               TEXT NOT NULL DEFAULT '{}',
  config                TEXT NOT NULL DEFAULT '{}',
  max_jobs_per_run      INTEGER NOT NULL DEFAULT 20 CHECK (max_jobs_per_run >= 1),
  starting_source_index INTEGER NOT NULL DEFAULT 0 CHECK (starting_source_index >= 0),
  last_run_at           TEXT,
  last_status           TEXT CHECK (last_status IN ('ok','success','error','empty')),
  last_outcome          TEXT CHECK (last_outcome IN ('running','success','success_with_warnings','partial','failed','cancelled')),
  last_error            TEXT,
  needs_review          INTEGER NOT NULL DEFAULT 0 CHECK (needs_review IN (0, 1)),
  created_at            TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflows_cadence ON workflows(cadence, enabled);

-- Backfill workflows from existing single-source workflows in sources table
INSERT INTO workflows (
  id, name, enabled, cadence, cadence_cron, next_run_at, retention_days, retention_policy,
  filters, config, max_jobs_per_run, starting_source_index, last_run_at, last_status,
  last_outcome, last_error, needs_review, created_at
)
SELECT
  id, name, enabled, cadence, cadence_cron, next_run_at, retention_days, retention_policy,
  filters, config, 20, 0, last_run_at, last_status,
  last_outcome, last_error, needs_review, created_at
FROM sources;

-- Alter sources to establish parent workflow ownership
ALTER TABLE sources ADD COLUMN workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE sources ADD COLUMN employer_hint TEXT;
ALTER TABLE sources ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;

-- Backfill sources.workflow_id
UPDATE sources SET workflow_id = id WHERE workflow_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_sources_workflow_id ON sources(workflow_id);

-- Alter jobs to establish workflow ownership
ALTER TABLE jobs ADD COLUMN workflow_id TEXT REFERENCES workflows(id) ON DELETE SET NULL;

-- Backfill jobs.workflow_id
UPDATE jobs SET workflow_id = source_id WHERE workflow_id IS NULL AND source_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_jobs_workflow_id ON jobs(workflow_id);

-- Alter scrape_runs to establish workflow ownership
ALTER TABLE scrape_runs ADD COLUMN workflow_id TEXT REFERENCES workflows(id) ON DELETE SET NULL;

-- Backfill scrape_runs.workflow_id
UPDATE scrape_runs SET workflow_id = source_id WHERE workflow_id IS NULL AND source_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_scrape_runs_workflow_id ON scrape_runs(workflow_id);
