PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sources (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('greenhouse','lever','ashby','jsonld','selector')),
  name         TEXT NOT NULL,
  identifier   TEXT NOT NULL,
  config       TEXT NOT NULL DEFAULT '{}',
  enabled      INTEGER NOT NULL DEFAULT 1,
  last_run_at  TEXT,
  last_status  TEXT CHECK (last_status IN ('ok','error','empty')),
  last_error   TEXT,
  created_at   TEXT NOT NULL,
  UNIQUE (kind, identifier)
);

CREATE TABLE IF NOT EXISTS jobs (
  id                 TEXT PRIMARY KEY,
  dedupe_key         TEXT NOT NULL UNIQUE,
  source_id          TEXT REFERENCES sources(id) ON DELETE SET NULL,
  external_id        TEXT,
  title              TEXT NOT NULL,
  company            TEXT NOT NULL,
  role_type          TEXT NOT NULL DEFAULT 'unknown'
                     CHECK (role_type IN ('full_time','contract','internship','part_time','unknown')),
  location           TEXT,
  url                TEXT NOT NULL,
  date_posted        TEXT,
  date_added         TEXT NOT NULL,
  last_seen_at       TEXT NOT NULL,
  status             TEXT NOT NULL DEFAULT 'scraped'
                     CHECK (status IN ('scraped','viewed','applied','interviewing','offer','rejected','ignored')),
  status_changed_at  TEXT NOT NULL,
  notes              TEXT
);

CREATE INDEX IF NOT EXISTS idx_jobs_status      ON jobs(status, date_added DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_role_type   ON jobs(role_type);

CREATE TABLE IF NOT EXISTS job_status_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id       TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  from_status  TEXT,
  to_status    TEXT NOT NULL,
  changed_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_to_status ON job_status_events(to_status, changed_at);

CREATE TABLE IF NOT EXISTS scrape_runs (
  id           TEXT PRIMARY KEY,
  trigger      TEXT NOT NULL CHECK (trigger IN ('schedule','manual','startup')),
  status       TEXT NOT NULL CHECK (status IN ('running','success','partial','failed')),
  started_at   TEXT NOT NULL,
  finished_at  TEXT,
  jobs_found   INTEGER NOT NULL DEFAULT 0,
  jobs_new     INTEGER NOT NULL DEFAULT 0,
  error        TEXT
);
