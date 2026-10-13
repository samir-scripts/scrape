-- Cache date-only provider dates interpreted in an explicit filter timezone.
-- The original date_posted evidence remains unchanged.
CREATE TABLE posting_date_cache (
    date_text TEXT NOT NULL,
    timezone TEXT NOT NULL,
    instant TEXT NOT NULL,
    PRIMARY KEY(date_text, timezone)
);
