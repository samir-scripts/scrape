-- Finalize posting-removal provenance without changing historical migration rows.
UPDATE jobs SET source_name_snapshot = (SELECT name FROM sources WHERE id = jobs.source_id)
WHERE source_name_snapshot IS NULL;
