-- SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
-- SPDX-License-Identifier: MPL-2.0

CREATE TABLE IF NOT EXISTS notes (
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
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS collections (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    name TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS records (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    collection_id BLOB NOT NULL
        REFERENCES collections(id) ON DELETE CASCADE
        CHECK (typeof(collection_id) = 'blob' AND length(collection_id) = 16),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS field_defs (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    name TEXT NOT NULL,
    type TEXT NOT NULL
        CHECK (type IN ('text', 'boolean', 'integer', 'date', 'time', 'timestamp')),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    UNIQUE (id, type)
) STRICT;

CREATE TABLE IF NOT EXISTS fields (
    record_id BLOB NOT NULL
        REFERENCES records(id) ON DELETE CASCADE
        CHECK (typeof(record_id) = 'blob' AND length(record_id) = 16),
    field_def_id BLOB NOT NULL
        CHECK (typeof(field_def_id) = 'blob' AND length(field_def_id) = 16),
    type TEXT NOT NULL
        CHECK (type IN ('text', 'boolean', 'integer', 'date', 'time', 'timestamp')),
    value ANY NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    PRIMARY KEY (record_id, field_def_id),
    FOREIGN KEY (field_def_id, type)
        REFERENCES field_defs(id, type) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS records_by_collection
ON records (collection_id, deleted, id);

CREATE INDEX IF NOT EXISTS fields_by_definition
ON fields (field_def_id, type);
