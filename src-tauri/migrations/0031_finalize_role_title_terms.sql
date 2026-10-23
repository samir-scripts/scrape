-- Converge both recognized migration-30 histories without relabeling either.
CREATE TABLE role_title_terms_repaired (
    term TEXT NOT NULL,
    role_title_id INTEGER NOT NULL REFERENCES role_titles(id) ON DELETE CASCADE,
    PRIMARY KEY (term, role_title_id)
);

INSERT INTO role_title_terms_repaired (term, role_title_id)
SELECT term, role_title_id FROM role_title_terms;

DROP TABLE role_title_terms;
ALTER TABLE role_title_terms_repaired RENAME TO role_title_terms;
