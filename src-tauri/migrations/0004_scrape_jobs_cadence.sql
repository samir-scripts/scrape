-- Migration 0004: Scrape jobs cadence, execution duration, and single source runs

PRAGMA foreign_keys = OFF;

-- Recreate sources table with cadence, cron, next_run_at, and duration
CREATE TABLE sources_new (
  id                    TEXT PRIMARY KEY,
  kind                  TEXT NOT NULL CHECK (kind IN ('greenhouse','lever','ashby','jsonld','selector')),
  name                  TEXT NOT NULL,
  identifier            TEXT NOT NULL,
  config                TEXT NOT NULL DEFAULT '{}',
  enabled               INTEGER NOT NULL DEFAULT 1,
  last_run_at           TEXT,
  last_status           TEXT CHECK (last_status IN ('ok','success','error','empty')),
  last_error            TEXT,
  created_at            TEXT NOT NULL,
  cadence               TEXT NOT NULL DEFAULT 'global'
                        CHECK (cadence IN ('global','hourly','every_6h','every_12h','daily','weekly','manual','cron')),
  cadence_cron          TEXT,
  next_run_at           TEXT,
  last_run_duration_ms  INTEGER,
  UNIQUE (kind, identifier)
);

INSERT INTO sources_new (id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at, cadence, cadence_cron, next_run_at, last_run_duration_ms)
SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at, 'global', NULL, NULL, NULL
FROM sources;

DROP TABLE sources;
ALTER TABLE sources_new RENAME TO sources;

CREATE INDEX IF NOT EXISTS idx_sources_cadence ON sources(cadence, enabled);

-- Recreate scrape_runs table to support 'single_source' trigger and optional source_id
CREATE TABLE scrape_runs_new (
  id           TEXT PRIMARY KEY,
  trigger      TEXT NOT NULL CHECK (trigger IN ('schedule','manual','startup','single_source')),
  status       TEXT NOT NULL CHECK (status IN ('running','success','partial','failed')),
  started_at   TEXT NOT NULL,
  finished_at  TEXT,
  jobs_found   INTEGER NOT NULL DEFAULT 0,
  jobs_new     INTEGER NOT NULL DEFAULT 0,
  error        TEXT,
  source_id    TEXT REFERENCES sources(id) ON DELETE SET NULL
);

INSERT INTO scrape_runs_new (id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error, source_id)
SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error, NULL
FROM scrape_runs;

DROP TABLE scrape_runs;
ALTER TABLE scrape_runs_new RENAME TO scrape_runs;

CREATE INDEX IF NOT EXISTS idx_scrape_runs_source_id ON scrape_runs(source_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_scrape_runs_started   ON scrape_runs(started_at DESC);

PRAGMA foreign_keys = ON;
