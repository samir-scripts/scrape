ALTER TABLE jobs ADD COLUMN posting_identity TEXT;
ALTER TABLE jobs ADD COLUMN removed_at TEXT;
ALTER TABLE jobs ADD COLUMN source_name_snapshot TEXT;
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
-- Every historical key remains individually reversible, even if several keys
-- resolve to the same posting identity.
CREATE TABLE posting_legacy_blocks (
    legacy_key TEXT PRIMARY KEY,
    identity TEXT NOT NULL
);
CREATE INDEX idx_legacy_block_identity ON posting_legacy_blocks(identity);
