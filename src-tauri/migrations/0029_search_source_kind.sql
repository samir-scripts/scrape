-- Add search to the allowed source kinds CHECK constraint.
-- SQLite requires table recreation to alter CHECK constraints;
-- preserve foreign-key referencing links while foreign keys are enabled.
CREATE TEMP TABLE source_job_links_29 AS SELECT id,source_id FROM jobs WHERE source_id IS NOT NULL;
CREATE TEMP TABLE source_run_links_29 AS SELECT id,source_id FROM scrape_runs WHERE source_id IS NOT NULL;
CREATE TABLE sources_next_29 (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK (kind IN ('greenhouse','lever','ashby','jsonld','selector','workday','search')),
 name TEXT NOT NULL,
 identifier TEXT NOT NULL,
 config TEXT NOT NULL DEFAULT '{}',
 enabled INTEGER NOT NULL DEFAULT 1,
 last_run_at TEXT,
 last_status TEXT CHECK (last_status IN ('ok','success','error','empty')),
 last_error TEXT,
 created_at TEXT NOT NULL,
 cadence TEXT NOT NULL DEFAULT 'global' CHECK (cadence IN ('global','hourly','every_6h','every_12h','daily','weekly','manual','cron')),
 cadence_cron TEXT,
 next_run_at TEXT,
 last_run_duration_ms INTEGER,
 retention_days INTEGER NOT NULL DEFAULT 30 CHECK (retention_days >= 0),
 retention_policy TEXT NOT NULL DEFAULT 'untracked_only' CHECK (retention_policy IN ('untracked_only','all')),
 filters TEXT NOT NULL DEFAULT '{}',
 company_icon_url TEXT,
 needs_review INTEGER NOT NULL DEFAULT 0 CHECK (needs_review IN (0,1)),
 last_outcome TEXT CHECK (last_outcome IN ('success','success_with_warnings','partial','failed','cancelled')),
 workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
 employer_hint TEXT,
 order_index INTEGER NOT NULL DEFAULT 0,
 script_trusted INTEGER NOT NULL DEFAULT 0,
 script_sha256 TEXT,
 UNIQUE(workflow_id,kind,identifier)
);
INSERT INTO sources_next_29 SELECT * FROM sources;
DROP TABLE sources;
ALTER TABLE sources_next_29 RENAME TO sources;
UPDATE jobs SET source_id=(SELECT source_id FROM source_job_links_29 WHERE id=jobs.id) WHERE id IN (SELECT id FROM source_job_links_29);
UPDATE scrape_runs SET source_id=(SELECT source_id FROM source_run_links_29 WHERE id=scrape_runs.id) WHERE id IN (SELECT id FROM source_run_links_29);
DROP TABLE source_job_links_29;
DROP TABLE source_run_links_29;
CREATE INDEX idx_sources_cadence ON sources(cadence,enabled);
CREATE INDEX idx_sources_workflow_id ON sources(workflow_id);
