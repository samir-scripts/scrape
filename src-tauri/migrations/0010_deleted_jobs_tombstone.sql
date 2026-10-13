-- Migration 0010: Tombstone table for deleted jobs
CREATE TABLE IF NOT EXISTS deleted_jobs (
  dedupe_key TEXT PRIMARY KEY,
  deleted_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deleted_jobs_time ON deleted_jobs(deleted_at);
