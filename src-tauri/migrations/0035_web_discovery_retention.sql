-- Preserve discovery observations and source identity when a source is removed.
CREATE TABLE discovery_observations_next_35 (
 job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
 run_id TEXT NOT NULL REFERENCES scrape_runs(id) ON DELETE CASCADE,
 source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
 source_key TEXT NOT NULL,
 observed_at TEXT NOT NULL,
 provenance TEXT NOT NULL CHECK (json_valid(provenance)),
 PRIMARY KEY (job_id, run_id, source_key)
);
INSERT INTO discovery_observations_next_35 (job_id, run_id, source_id, source_key, observed_at, provenance)
 SELECT job_id, run_id, source_id, source_id, observed_at, provenance FROM discovery_observations;
DROP TABLE discovery_observations;
ALTER TABLE discovery_observations_next_35 RENAME TO discovery_observations;
CREATE INDEX idx_discovery_observations_run ON discovery_observations(run_id);

-- Recognized early v34 databases used the older non-search uniqueness predicate.
DROP INDEX idx_sources_non_search_endpoint;
CREATE UNIQUE INDEX idx_sources_non_search_endpoint ON sources(workflow_id,kind,identifier) WHERE kind NOT IN ('search','web_discovery');
