-- Migration 0006: App notifications storage
CREATE TABLE IF NOT EXISTS app_notifications (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('update', 'news', 'system', 'scraper')),
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    action_label TEXT,
    action_payload TEXT,
    read INTEGER NOT NULL DEFAULT 0 CHECK (read IN (0, 1)),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notifications_read ON app_notifications (read);
CREATE INDEX IF NOT EXISTS idx_notifications_created ON app_notifications (created_at DESC);
