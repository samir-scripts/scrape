CREATE TABLE role_titles (
    id INTEGER PRIMARY KEY,
    soc_code TEXT NOT NULL,
    title TEXT NOT NULL,
    priority_tier INTEGER NOT NULL CHECK (priority_tier BETWEEN 1 AND 3)
);

CREATE UNIQUE INDEX idx_role_titles_title_nocase ON role_titles(title COLLATE NOCASE);
CREATE INDEX idx_role_titles_tier_title ON role_titles(priority_tier, title COLLATE NOCASE);

CREATE TABLE role_title_terms (
    term TEXT NOT NULL,
    role_title_id INTEGER NOT NULL REFERENCES role_titles(id) ON DELETE CASCADE,
    PRIMARY KEY (term, role_title_id)
) WITHOUT ROWID;
