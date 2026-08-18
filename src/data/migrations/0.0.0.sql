-- SPDX-License-Identifier: MPL-2.0

CREATE TABLE notes (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    body TEXT NOT NULL,
    revision INTEGER NOT NULL
        CHECK (
            typeof(revision) = 'integer'
            AND revision BETWEEN 1 AND 4294967295
        ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL CHECK (updated_at >= created_at),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1))
) STRICT;
