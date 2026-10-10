-- Migration 0003: Add 'seasonal' to jobs role_type CHECK constraint

CREATE TABLE jobs_new (
  id                 TEXT PRIMARY KEY,
  dedupe_key         TEXT NOT NULL UNIQUE,
  source_id          TEXT REFERENCES sources(id) ON DELETE SET NULL,
  external_id        TEXT,
  title              TEXT NOT NULL,
  company            TEXT NOT NULL,
  role_type          TEXT NOT NULL DEFAULT 'unknown'
                     CHECK (role_type IN ('full_time','contract','internship','part_time','seasonal','unknown')),
  location           TEXT,
  url                TEXT NOT NULL,
  date_posted        TEXT,
  date_added         TEXT NOT NULL,
  last_seen_at       TEXT NOT NULL,
  status             TEXT NOT NULL DEFAULT 'scraped'
                     CHECK (status IN ('scraped','viewed','applied','interviewing','offer','rejected','ignored')),
  status_changed_at  TEXT NOT NULL,
  notes              TEXT,
  salary_min         INTEGER,
  salary_max         INTEGER,
  salary_currency    TEXT,
  salary_interval    TEXT CHECK (salary_interval IN ('year','month','week','day','hour')),
  salary_raw         TEXT,
  salary_source      TEXT CHECK (salary_source IN ('structured','text')),
  workplace_type     TEXT NOT NULL DEFAULT 'unknown'
                     CHECK (workplace_type IN ('remote','hybrid','onsite','unknown')),
  sponsorship        TEXT NOT NULL DEFAULT 'unknown' CHECK (sponsorship IN ('yes','no','unknown')),
  sponsorship_evidence TEXT,
  clearance          TEXT NOT NULL DEFAULT 'unknown' CHECK (clearance IN ('yes','no','unknown')),
  clearance_level    TEXT CHECK (clearance_level IN ('public_trust','confidential','secret','top_secret','ts_sci')),
  clearance_evidence TEXT,
  experience_min     INTEGER,
  experience_max     INTEGER,
  experience_raw     TEXT,
  is_new_grad        TEXT NOT NULL DEFAULT 'unknown' CHECK (is_new_grad IN ('yes','no','unknown')),
  experience_evidence TEXT,
  description_text   TEXT,
  enrichment_version INTEGER NOT NULL DEFAULT 0
);

INSERT INTO jobs_new SELECT
  id, dedupe_key, source_id, external_id, title, company, role_type,
  location, url, date_posted, date_added, last_seen_at, status,
  status_changed_at, notes, salary_min, salary_max, salary_currency,
  salary_interval, salary_raw, salary_source, workplace_type,
  sponsorship, sponsorship_evidence, clearance, clearance_level,
  clearance_evidence, experience_min, experience_max, experience_raw,
  is_new_grad, experience_evidence, description_text, enrichment_version
FROM jobs;

DROP TABLE jobs;

ALTER TABLE jobs_new RENAME TO jobs;

CREATE INDEX IF NOT EXISTS idx_jobs_status      ON jobs(status, date_added DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_role_type   ON jobs(role_type);
CREATE INDEX IF NOT EXISTS idx_jobs_salary       ON jobs(salary_currency, salary_max);
CREATE INDEX IF NOT EXISTS idx_jobs_eligibility  ON jobs(sponsorship, clearance);
CREATE INDEX IF NOT EXISTS idx_jobs_experience   ON jobs(experience_min, experience_max);
CREATE INDEX IF NOT EXISTS idx_jobs_new_grad     ON jobs(is_new_grad);
