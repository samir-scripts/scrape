-- Isolated browser session metadata and custom script trust state.
-- Forward-only additive migration; preserves sources.kind CHECK constraint.
CREATE TABLE IF NOT EXISTS browser_sessions (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    domain TEXT NOT NULL,
    storage_relpath TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ready'
        CHECK (status IN ('ready', 'needs_login', 'expired')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_verified_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_browser_sessions_domain ON browser_sessions(domain);
ALTER TABLE sources ADD COLUMN script_trusted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sources ADD COLUMN script_sha256 TEXT;
