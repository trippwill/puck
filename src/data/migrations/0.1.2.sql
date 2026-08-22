-- SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
-- SPDX-License-Identifier: MPL-2.0

ALTER TABLE records
ADD COLUMN label TEXT NOT NULL DEFAULT 'Untitled record'
CHECK (length(trim(label)) > 0);

ALTER TABLE records
ADD COLUMN source_note_id BLOB
REFERENCES notes(id) ON DELETE SET NULL
CHECK (
    source_note_id IS NULL
    OR (typeof(source_note_id) = 'blob' AND length(source_note_id) = 16)
);

CREATE INDEX records_by_source_note
ON records (source_note_id)
WHERE source_note_id IS NOT NULL;
