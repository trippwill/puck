-- SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
-- SPDX-License-Identifier: MPL-2.0

CREATE INDEX records_by_collection
ON records (collection_id, deleted, id);

CREATE INDEX fields_by_definition
ON fields (field_def_id, type);
