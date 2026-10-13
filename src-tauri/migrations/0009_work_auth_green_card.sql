-- Migration 0009: Work Authorization and Green Card classification

ALTER TABLE jobs ADD COLUMN work_auth TEXT NOT NULL DEFAULT 'unknown'
    CHECK (work_auth IN ('citizen_only', 'us_person', 'open', 'unknown'));
ALTER TABLE jobs ADD COLUMN work_auth_evidence TEXT;

CREATE INDEX IF NOT EXISTS idx_jobs_work_auth ON jobs(work_auth);
