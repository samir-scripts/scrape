-- Provider edit time is evidence distinct from publication and discovery dates.
ALTER TABLE jobs ADD COLUMN provider_updated_at TEXT;
