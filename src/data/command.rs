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
    /// Returns an archived note to the active pile.
    UnarchiveNote(PileNote),
    /// Creates or updates a collection.
    UpsertCollection(Collection),
    /// Creates or updates a record.
    UpsertRecord(Record),
    /// Creates or updates a field definition.
    UpsertFieldDef(AnyFieldDef),
    /// Creates or updates a field value.
    UpsertField(AnyField),
    /// Marks a collection and its contents for deletion.
    DeleteCollection(CollectionId),
    /// Marks a record and its fields for deletion.
    DeleteRecord(RecordId),
    /// Marks a field definition and its values for deletion.
    DeleteFieldDef(FieldDefId),
    /// Marks a field value for deletion.
    DeleteField(FieldKey),
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
            Command::UnarchiveNote(note) => Command::set_archived(tx, note.id(), false),
            Command::UpsertCollection(collection) => Command::upsert_collection(tx, &collection),
            Command::UpsertRecord(record) => Command::upsert_record(tx, &record),
            Command::UpsertFieldDef(field_def) => Command::upsert_field_def(tx, &field_def),
            Command::UpsertField(field) => Command::upsert_field(tx, &field),
            Command::DeleteCollection(id) => Command::delete_collection(tx, id),
            Command::DeleteRecord(id) => Command::delete_record(tx, id),
            Command::DeleteFieldDef(id) => Command::delete_field_def(tx, id),
            Command::DeleteField(key) => Command::delete_field(tx, key),
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
                note.created_at(),
                note.updated_at()
            ],
        )
    }

    fn edit_note(tx: &Transaction, note: &PileNote) -> SqlResult<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET body = ?2, revision = ?3, updated_at = ?4
            WHERE id = ?1 AND archived = 0
            ",
            params![
                *note.id().as_uuid(),
                note.body(),
                note.revision(),
                note.updated_at()
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
            WHERE id = ?1 AND archived != ?2
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

    fn upsert_collection(tx: &Transaction, collection: &Collection) -> SqlResult<usize> {
        tx.execute(
            r"
            INSERT INTO collections (id, name)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name
            ",
            params![*collection.id().as_uuid(), collection.name()],
        )
    }

    fn upsert_record(tx: &Transaction, record: &Record) -> SqlResult<usize> {
        tx.execute(
            r"
            INSERT INTO records (id, collection_id)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                collection_id = excluded.collection_id
            ",
            params![*record.id().as_uuid(), *record.collection_id().as_uuid()],
        )
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

        tx.execute(
            r"
            INSERT INTO field_defs (id, name, type)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id, type) DO UPDATE SET
                name = excluded.name
            ",
            params![*field_def.id().as_uuid(), field_def.name(), kind],
        )
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

        tx.execute(
            r"
            INSERT INTO fields (record_id, field_def_id, type, value)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(record_id, field_def_id) DO UPDATE SET
                type = excluded.type,
                value = excluded.value
            ",
            params![
                *field.record_id().as_uuid(),
                *field.def_id().as_uuid(),
                kind,
                value
            ],
        )
    }

    fn delete_collection(tx: &Transaction, id: CollectionId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE collections SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)?;
        tx.execute(
            r"
            UPDATE fields
            SET deleted = 1
            WHERE record_id IN (
                SELECT id FROM records WHERE collection_id = ?1
            )
            ",
            [*id.as_uuid()],
        )?;
        tx.execute(
            "UPDATE records SET deleted = 1 WHERE collection_id = ?1",
            [*id.as_uuid()],
        )?;
        Ok(changed)
    }

    fn delete_record(tx: &Transaction, id: RecordId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE records SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)?;
        tx.execute(
            "UPDATE fields SET deleted = 1 WHERE record_id = ?1",
            [*id.as_uuid()],
        )?;
        Ok(changed)
    }

    fn delete_field_def(tx: &Transaction, id: FieldDefId) -> SqlResult<usize> {
        let changed = tx.execute(
            "UPDATE field_defs SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            [*id.as_uuid()],
        )?;
        require_one(changed)?;
        tx.execute(
            "UPDATE fields SET deleted = 1 WHERE field_def_id = ?1",
            [*id.as_uuid()],
        )?;
        Ok(changed)
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
