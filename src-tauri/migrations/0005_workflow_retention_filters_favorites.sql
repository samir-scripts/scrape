-- Migration 0005: Workflow retention, per-source filters, and job favoriting

ALTER TABLE sources ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 30 CHECK (retention_days >= 0);
ALTER TABLE sources ADD COLUMN retention_policy TEXT NOT NULL DEFAULT 'untracked_only' CHECK (retention_policy IN ('untracked_only', 'all'));
ALTER TABLE sources ADD COLUMN filters TEXT NOT NULL DEFAULT '{}';

ALTER TABLE jobs ADD COLUMN favorited INTEGER NOT NULL DEFAULT 0 CHECK (favorited IN (0, 1));
ALTER TABLE jobs ADD COLUMN favorited_at TEXT;

CREATE INDEX IF NOT EXISTS idx_jobs_favorited ON jobs(favorited, date_added DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_source_retention ON jobs(source_id, status, favorited, date_added);
