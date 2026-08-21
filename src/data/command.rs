// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use tokio_rusqlite::Transaction;
use tokio_rusqlite::rusqlite::{Error as SqlError, Result as SqlResult, params};

use super::{AnyFieldValue, SqlFieldType};
use crate::core::prelude::*;

/// A command to be executed on the document.
#[derive(Debug)]
pub enum Command {
    /// Adds a pile note to the document.
    AddNote(PileNote),
    /// Moves a note out of the active pile.
    ArchiveNote(ArchiveNote),
    /// Persists an edited pile note.
    EditNote(PileNote),
    /// Marks an archived note for deletion.
    DeleteNote(NoteId),
    /// Restores a deleted note to the archive.
    UndeleteNote(NoteId),
    /// Returns an archived note to the active pile.
    UnarchiveNote(PileNote),
    /// Creates or updates a collection.
    UpsertCollection(Collection),
    /// Creates or updates a record.
    UpsertRecord(Record),
    /// Moves a record to an active collection.
    MoveRecord(RecordId, CollectionId),
    /// Creates or updates a field definition.
    UpsertFieldDef(AnyFieldDef),
    /// Creates or updates a field value.
    UpsertField(AnyField),
    /// Marks a collection for deletion.
    DeleteCollection(CollectionId),
    /// Restores a deleted collection.
    UndeleteCollection(CollectionId),
    /// Marks a record for deletion.
    DeleteRecord(RecordId),
    /// Restores a deleted record whose collection is active.
    UndeleteRecord(RecordId),
    /// Marks a field definition for deletion.
    DeleteFieldDef(FieldDefId),
    /// Restores a deleted field definition.
    UndeleteFieldDef(FieldDefId),
    /// Marks a field value for deletion.
    DeleteField(FieldKey),
    /// Restores a deleted field whose record and definition are active.
    UndeleteField(FieldKey),
    /// Permanently removes structured data marked for deletion.
    Clean,
}

impl Command {
    pub(crate) fn execute(self, tx: &Transaction) -> SqlResult<usize> {
        match self {
            Command::AddNote(note) => Command::add_note(tx, &note),
            Command::ArchiveNote(note) => Command::set_archived(tx, note.id(), true),
            Command::EditNote(note) => Command::edit_note(tx, &note),
            Command::DeleteNote(id) => Command::delete_note(tx, id),
            Command::UndeleteNote(id) => Command::undelete_note(tx, id),
            Command::UnarchiveNote(note) => Command::set_archived(tx, note.id(), false),
            Command::UpsertCollection(collection) => Command::upsert_collection(tx, &collection),
            Command::UpsertRecord(record) => Command::upsert_record(tx, &record),
            Command::MoveRecord(record, collection) => Command::move_record(tx, record, collection),
            Command::UpsertFieldDef(field_def) => Command::upsert_field_def(tx, &field_def),
            Command::UpsertField(field) => Command::upsert_field(tx, &field),
            Command::DeleteCollection(id) => Command::delete_collection(tx, id),
            Command::UndeleteCollection(id) => Command::undelete_collection(tx, id),
            Command::DeleteRecord(id) => Command::delete_record(tx, id),
            Command::UndeleteRecord(id) => Command::undelete_record(tx, id),
            Command::DeleteFieldDef(id) => Command::delete_field_def(tx, id),
            Command::UndeleteFieldDef(id) => Command::undelete_field_def(tx, id),
            Command::DeleteField(key) => Command::delete_field(tx, key),
            Command::UndeleteField(key) => Command::undelete_field(tx, key),
            Command::Clean => Command::clean(tx),
        }
    }

    fn add_note(tx: &Transaction, note: &PileNote) -> SqlResult<usize> {
        tx.execute(
            r"
            INSERT INTO notes (id, body, revision, created_at, updated_at, archived)
            VALUES (?1, ?2, ?3, ?4, ?5, 0)
            ",
            params![
                *note.id().as_uuid(),
                note.body().to_owned(),
                note.revision(),
                note.created_at().as_milliseconds(),
                note.updated_at().as_milliseconds()
            ],
        )
    }

    fn edit_note(tx: &Transaction, note: &PileNote) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET body = ?2, revision = ?3, updated_at = ?4
            WHERE id = ?1 AND archived = 0 AND deleted = 0
            ",
            params![
                *note.id().as_uuid(),
                note.body(),
                note.revision(),
                note.updated_at().as_milliseconds()
            ],
        )?;

        match changed {
            1 => Ok(changed),
            _ => Err(SqlError::QueryReturnedNoRows),
        }
    }

    fn set_archived(tx: &Transaction, id: NoteId, archived: bool) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET archived = ?2
            WHERE id = ?1 AND archived != ?2 AND deleted = 0
            ",
            params![*id.as_uuid(), archived],
        )?;

        match changed {
            1 => Ok(changed),
            _ => Err(SqlError::QueryReturnedNoRows),
        }
    }

    fn delete_note(tx: &Transaction, id: NoteId) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET deleted = 1
            WHERE id = ?1 AND archived = 1 AND deleted = 0
            ",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn undelete_note(tx: &Transaction, id: NoteId) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET deleted = 0
            WHERE id = ?1 AND archived = 1 AND deleted = 1
            ",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn upsert_collection(tx: &Transaction, collection: &Collection) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            INSERT INTO collections (id, name)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name
            WHERE collections.deleted = 0
            ",
            params![*collection.id().as_uuid(), collection.name()],
        )?;
        require_one(changed)
    }

    fn upsert_record(tx: &Transaction, record: &Record) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            INSERT INTO records (id, collection_id)
            SELECT ?1, ?2
            WHERE EXISTS (
                SELECT 1
                FROM collections
                WHERE id = ?2 AND deleted = 0
            )
            ON CONFLICT(id) DO UPDATE SET
                collection_id = excluded.collection_id
            WHERE records.deleted = 0
            ",
            params![*record.id().as_uuid(), *record.collection_id().as_uuid()],
        )?;
        require_one(changed)
    }

    fn move_record(
        tx: &Transaction,
        record: RecordId,
        collection: CollectionId,
    ) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE records
            SET collection_id = ?2
            WHERE id = ?1
                AND deleted = 0
                AND EXISTS (
                    SELECT 1
                    FROM collections
                    WHERE id = records.collection_id AND deleted = 0
                )
                AND EXISTS (
                    SELECT 1
                    FROM collections
                    WHERE id = ?2 AND deleted = 0
                )
            ",
            params![*record.as_uuid(), *collection.as_uuid()],
        )?;
        require_one(changed)
    }

    fn upsert_field_def(tx: &Transaction, field_def: &AnyFieldDef) -> SqlResult<usize> {
        let kind = match field_def {
            AnyFieldDef::Text(_) => SqlFieldType::Text,
            AnyFieldDef::Boolean(_) => SqlFieldType::Boolean,
            AnyFieldDef::Integer(_) => SqlFieldType::Integer,
            AnyFieldDef::Date(_) => SqlFieldType::Date,
            AnyFieldDef::Time(_) => SqlFieldType::Time,
            AnyFieldDef::Timestamp(_) => SqlFieldType::Timestamp,
        };

        let changed = tx.execute(
            r"
            INSERT INTO field_defs (id, name, type)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id, type) DO UPDATE SET
                name = excluded.name
            WHERE field_defs.deleted = 0
            ",
            params![*field_def.id().as_uuid(), field_def.name(), kind],
        )?;
        require_one(changed)
    }

    fn upsert_field(tx: &Transaction, field: &AnyField) -> SqlResult<usize> {
        let (kind, value) = match field {
            AnyField::Text(f) => (SqlFieldType::Text, AnyFieldValue::Text(f.val().to_owned())),
            AnyField::Boolean(f) => (SqlFieldType::Boolean, AnyFieldValue::Boolean(*f.val())),
            AnyField::Integer(f) => (SqlFieldType::Integer, AnyFieldValue::Integer(*f.val())),
            AnyField::Date(f) => (SqlFieldType::Date, AnyFieldValue::Date(f.val().to_owned())),
            AnyField::Time(f) => (SqlFieldType::Time, AnyFieldValue::Time(f.val().to_owned())),
            AnyField::Timestamp(f) => (
                SqlFieldType::Timestamp,
                AnyFieldValue::Timestamp(f.val().as_milliseconds()),
            ),
        };

        let changed = tx.execute(
            r"
            INSERT INTO fields (record_id, field_def_id, type, value)
            SELECT ?1, ?2, ?3, ?4
            WHERE EXISTS (
                SELECT 1
                FROM records
                JOIN collections ON collections.id = records.collection_id
                WHERE records.id = ?1
                    AND records.deleted = 0
                    AND collections.deleted = 0
            )
            AND EXISTS (
                SELECT 1
                FROM field_defs
                WHERE id = ?2 AND type = ?3 AND deleted = 0
            )
            ON CONFLICT(record_id, field_def_id) DO UPDATE SET
                type = excluded.type,
                value = excluded.value
            WHERE fields.deleted = 0
            ",
            params![
                *field.record_id().as_uuid(),
                *field.def_id().as_uuid(),
                kind,
                value
            ],
        )?;
        require_one(changed)
    }

    fn delete_collection(tx: &Transaction, id: CollectionId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE collections SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn undelete_collection(tx: &Transaction, id: CollectionId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE collections SET deleted = 0 WHERE id = ?1 AND deleted = 1",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn delete_record(tx: &Transaction, id: RecordId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE records SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn undelete_record(tx: &Transaction, id: RecordId) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE records
            SET deleted = 0
            WHERE id = ?1
                AND deleted = 1
                AND EXISTS (
                    SELECT 1
                    FROM collections
                    WHERE collections.id = records.collection_id
                        AND collections.deleted = 0
                )
            ",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn delete_field_def(tx: &Transaction, id: FieldDefId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE field_defs SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn undelete_field_def(tx: &Transaction, id: FieldDefId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE field_defs SET deleted = 0 WHERE id = ?1 AND deleted = 1",
            [*id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn delete_field(tx: &Transaction, key: FieldKey) -> SqlResult<usize> {
        let FieldKey(record_id, def_id) = key;
        let changed = tx.execute(
            r"
            UPDATE fields
            SET deleted = 1
            WHERE record_id = ?1 AND field_def_id = ?2 AND deleted = 0
            ",
            params![*record_id.as_uuid(), *def_id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn undelete_field(tx: &Transaction, key: FieldKey) -> SqlResult<usize> {
        let FieldKey(record_id, def_id) = key;
        let changed = tx.execute(
            r"
            UPDATE fields
            SET deleted = 0
            WHERE record_id = ?1
                AND field_def_id = ?2
                AND deleted = 1
                AND EXISTS (
                    SELECT 1
                    FROM records
                    JOIN collections ON collections.id = records.collection_id
                    WHERE records.id = ?1
                        AND records.deleted = 0
                        AND collections.deleted = 0
                )
                AND EXISTS (
                    SELECT 1
                    FROM field_defs
                    WHERE field_defs.id = ?2
                        AND field_defs.deleted = 0
                )
            ",
            params![*record_id.as_uuid(), *def_id.as_uuid()],
        )?;
        require_one(changed)
    }

    fn clean(tx: &Transaction) -> SqlResult<usize> {
        let mut changed = tx.execute("DELETE FROM notes WHERE deleted = 1", [])?;
        changed += tx.execute("DELETE FROM fields WHERE deleted = 1", [])?;
        changed += tx.execute("DELETE FROM records WHERE deleted = 1", [])?;
        changed += tx.execute("DELETE FROM field_defs WHERE deleted = 1", [])?;
        changed += tx.execute("DELETE FROM collections WHERE deleted = 1", [])?;
        tx.execute_batch("PRAGMA optimize")?;
        Ok(changed)
    }
}

fn require_one(changed: usize) -> SqlResult<usize> {
    match changed {
        1 => Ok(changed),
        _ => Err(SqlError::QueryReturnedNoRows),
    }
}
