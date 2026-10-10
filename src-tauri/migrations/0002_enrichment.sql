-- Migration 0002: Job Enrichment, Locations, Eligibility, Experience, and Scraper Logs

-- 1. Extend jobs table with structured enrichment fields
ALTER TABLE jobs ADD COLUMN salary_min        INTEGER;               -- annualized, whole currency units
ALTER TABLE jobs ADD COLUMN salary_max        INTEGER;               -- annualized
ALTER TABLE jobs ADD COLUMN salary_currency   TEXT;                  -- ISO 4217 (USD, CAD, GBP, EUR...)
ALTER TABLE jobs ADD COLUMN salary_interval   TEXT CHECK (salary_interval IN ('year','month','week','day','hour'));
ALTER TABLE jobs ADD COLUMN salary_raw        TEXT;                  -- original unparsed string for display
ALTER TABLE jobs ADD COLUMN salary_source     TEXT CHECK (salary_source IN ('structured','text'));
ALTER TABLE jobs ADD COLUMN workplace_type    TEXT NOT NULL DEFAULT 'unknown'
                         CHECK (workplace_type IN ('remote','hybrid','onsite','unknown'));
ALTER TABLE jobs ADD COLUMN sponsorship       TEXT NOT NULL DEFAULT 'unknown' CHECK (sponsorship IN ('yes','no','unknown'));
ALTER TABLE jobs ADD COLUMN sponsorship_evidence TEXT;
ALTER TABLE jobs ADD COLUMN clearance         TEXT NOT NULL DEFAULT 'unknown' CHECK (clearance IN ('yes','no','unknown'));
ALTER TABLE jobs ADD COLUMN clearance_level   TEXT CHECK (clearance_level IN ('public_trust','confidential','secret','top_secret','ts_sci'));
ALTER TABLE jobs ADD COLUMN clearance_evidence TEXT;
ALTER TABLE jobs ADD COLUMN experience_min    INTEGER;               -- minimum required years of experience (0 for new grad/entry)
ALTER TABLE jobs ADD COLUMN experience_max    INTEGER;               -- maximum years (e.g. 5 for "3-5 years", null for "5+")
ALTER TABLE jobs ADD COLUMN experience_raw    TEXT;                  -- e.g. "3-5 years", "5+ years", "0-1 years"
ALTER TABLE jobs ADD COLUMN is_new_grad       TEXT NOT NULL DEFAULT 'unknown' CHECK (is_new_grad IN ('yes','no','unknown'));
ALTER TABLE jobs ADD COLUMN experience_evidence TEXT;                -- matching snippet/quote
ALTER TABLE jobs ADD COLUMN description_text  TEXT;                  -- plain text description, capped
ALTER TABLE jobs ADD COLUMN enrichment_version INTEGER NOT NULL DEFAULT 0;

-- 2. Normalized locations child table (supports multi-location listings)
CREATE TABLE IF NOT EXISTS job_locations (
  job_id        TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  raw           TEXT NOT NULL,
  city          TEXT,
  region_code   TEXT,             -- ISO 3166-2, e.g. 'US-CA', 'CA-ON'
  country_code  TEXT,             -- ISO 3166-1 alpha-2, e.g. 'US'
  is_remote     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (job_id, raw)
);

-- 3. Scraper execution granular event logs
CREATE TABLE IF NOT EXISTS scrape_logs (
  id           TEXT PRIMARY KEY,
  run_id       TEXT NOT NULL REFERENCES scrape_runs(id) ON DELETE CASCADE,
  timestamp    TEXT NOT NULL,
  level        TEXT NOT NULL CHECK (level IN ('info','warn','error')),
  source_id    TEXT,
  message      TEXT NOT NULL
);

-- 4. Indexes for high-performance query-time filtering
CREATE INDEX IF NOT EXISTS idx_job_locations_geo ON job_locations(country_code, region_code);
CREATE INDEX IF NOT EXISTS idx_jobs_salary       ON jobs(salary_currency, salary_max);
CREATE INDEX IF NOT EXISTS idx_jobs_eligibility  ON jobs(sponsorship, clearance);
CREATE INDEX IF NOT EXISTS idx_jobs_experience   ON jobs(experience_min, experience_max);
CREATE INDEX IF NOT EXISTS idx_jobs_new_grad     ON jobs(is_new_grad);
CREATE INDEX IF NOT EXISTS idx_scrape_logs_run   ON scrape_logs(run_id, timestamp);
