-- SPDX-License-Identifier: MPL-2.0

CREATE TABLE collections (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    name TEXT NOT NULL
) STRICT;

CREATE TABLE records (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    collection_id BLOB NOT NULL
        REFERENCES collections(id) ON DELETE CASCADE
        CHECK (typeof(collection_id) = 'blob' AND length(collection_id) = 16)
) STRICT;

CREATE TABLE field_defs (
    id BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(id) = 'blob' AND length(id) = 16),
    name TEXT NOT NULL,
    type TEXT NOT NULL
        CHECK (type IN ('text', 'boolean', 'integer', 'date', 'time', 'timestamp')),
    UNIQUE (id, type)
) STRICT;

CREATE TABLE fields (
    record_id BLOB NOT NULL
        REFERENCES records(id) ON DELETE CASCADE
        CHECK (typeof(record_id) = 'blob' AND length(record_id) = 16),
    field_def_id BLOB NOT NULL
        CHECK (typeof(field_def_id) = 'blob' AND length(field_def_id) = 16),
    type TEXT NOT NULL
        CHECK (type IN ('text', 'boolean', 'integer', 'date', 'time', 'timestamp')),
    value ANY NOT NULL,
    PRIMARY KEY (record_id, field_def_id),
    FOREIGN KEY (field_def_id, type)
        REFERENCES field_defs(id, type) ON DELETE CASCADE
) STRICT;
