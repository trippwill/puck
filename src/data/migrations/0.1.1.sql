-- SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
-- SPDX-License-Identifier: MPL-2.0

CREATE TABLE integer_time_notes (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    body TEXT NOT NULL,
    revision INTEGER NOT NULL
        CHECK (
            typeof(revision) = 'integer'
            AND revision BETWEEN 1 AND 4294967295
        ),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1))
) STRICT;

INSERT INTO integer_time_notes
SELECT
    id,
    body,
    revision,
    CAST(unixepoch(created_at, 'subsec') * 1000 AS INTEGER),
    CAST(unixepoch(updated_at, 'subsec') * 1000 AS INTEGER),
    archived,
    deleted
FROM notes;

DROP TABLE notes;

ALTER TABLE integer_time_notes RENAME TO notes;
