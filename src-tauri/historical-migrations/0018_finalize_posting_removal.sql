-- Forward repair for the exact historical version-12 schema only.
ALTER TABLE jobs ADD COLUMN source_name_snapshot TEXT;
CREATE TABLE posting_legacy_blocks (
    legacy_key TEXT PRIMARY KEY,
    identity TEXT NOT NULL
);
CREATE INDEX idx_legacy_block_identity ON posting_legacy_blocks(identity);
-- Finalize posting-removal provenance without changing historical migration rows.
UPDATE jobs SET source_name_snapshot = (SELECT name FROM sources WHERE id = jobs.source_id)
WHERE source_name_snapshot IS NULL;
