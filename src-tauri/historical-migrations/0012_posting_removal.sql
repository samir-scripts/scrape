ALTER TABLE jobs ADD COLUMN posting_identity TEXT;
ALTER TABLE jobs ADD COLUMN removed_at TEXT;
CREATE INDEX idx_jobs_posting_identity ON jobs(posting_identity);
CREATE INDEX idx_jobs_active ON jobs(removed_at, status, date_added);
CREATE TABLE blocked_postings (
    identity TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    company TEXT NOT NULL,
    url TEXT,
    source_name TEXT,
    blocked_at TEXT NOT NULL,
    legacy_key TEXT
);
CREATE INDEX idx_blocked_legacy ON blocked_postings(legacy_key);
