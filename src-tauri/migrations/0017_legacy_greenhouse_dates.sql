-- The pre-0011 Greenhouse adapter wrote updated_at into date_posted.
-- Only recognize records last seen before that upgrade; subsequent refreshed
-- records may already have genuine first_published evidence and remain intact.
-- Preserve the original provider edit timestamp while clearing the false posting date.
UPDATE jobs
SET provider_updated_at = date_posted, date_posted = NULL
WHERE source_id IN (SELECT id FROM sources WHERE kind = 'greenhouse')
  AND date_posted IS NOT NULL
  AND provider_updated_at IS NULL
  AND julianday(last_seen_at) < julianday(
    (SELECT installed_on FROM _sqlx_migrations WHERE version = 11)
  );
