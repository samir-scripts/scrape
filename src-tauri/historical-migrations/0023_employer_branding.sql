-- One verified asset per source/employer relationship. Keep the last attempt
-- separately so temporary network/decoding failures cannot erase verified art.
CREATE TABLE employer_branding (
    key TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    company TEXT NOT NULL,
    logo_json TEXT NOT NULL,
    last_attempt_json TEXT NOT NULL,
    asset_reference TEXT,
    updated_at TEXT NOT NULL
);
-- Intentionally independent of source deletion: saved postings retain artwork.
ALTER TABLE jobs ADD COLUMN company_branding_key TEXT;
