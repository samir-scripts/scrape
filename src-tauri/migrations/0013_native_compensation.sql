-- Original native compensation is separate from the historical annual analytics fields.
ALTER TABLE jobs ADD COLUMN compensation_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE jobs ADD COLUMN compensation_migrated INTEGER NOT NULL DEFAULT 0;
