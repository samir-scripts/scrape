-- This recognized v34 already has the retained observation schema.
DROP INDEX idx_sources_non_search_endpoint;
CREATE UNIQUE INDEX idx_sources_non_search_endpoint ON sources(workflow_id,kind,identifier) WHERE kind NOT IN ('search','web_discovery');
