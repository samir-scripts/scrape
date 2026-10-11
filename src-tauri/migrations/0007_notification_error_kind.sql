-- Migration 0007: Add 'error' to app_notifications kind CHECK constraint

CREATE TABLE IF NOT EXISTS app_notifications_new (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('update', 'news', 'system', 'scraper', 'error')),
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    action_label TEXT,
    action_payload TEXT,
    read INTEGER NOT NULL DEFAULT 0 CHECK (read IN (0, 1)),
    created_at TEXT NOT NULL
);

INSERT OR IGNORE INTO app_notifications_new (id, kind, title, message, action_label, action_payload, read, created_at)
SELECT id, kind, title, message, action_label, action_payload, read, created_at
FROM app_notifications;

DROP TABLE IF EXISTS app_notifications;

ALTER TABLE app_notifications_new RENAME TO app_notifications;

CREATE INDEX IF NOT EXISTS idx_notifications_read ON app_notifications (read);
CREATE INDEX IF NOT EXISTS idx_notifications_created ON app_notifications (created_at DESC);
