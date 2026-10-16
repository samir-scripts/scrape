-- Preserve existing run/source rows and CHECK constraints. New outcomes use
-- additive columns; historical rows retain their original status and NULL outcome.
ALTER TABLE scrape_runs ADD COLUMN outcome TEXT
    CHECK (outcome IN ('running','success','success_with_warnings','partial','failed','cancelled'));
ALTER TABLE sources ADD COLUMN last_outcome TEXT
    CHECK (last_outcome IN ('success','success_with_warnings','partial','failed','cancelled'));
