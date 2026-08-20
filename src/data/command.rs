// SPDX-License-Identifier: MPL-2.0

use rusqlite::params;
use tokio_rusqlite::rusqlite;

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
    /// Returns an archived note to the active pile.
    UnarchiveNote(PileNote),
    /// Creates or updates a collection.
    UpsertCollection(Collection),
    /// Creates or updates a record.
    UpsertRecord(Record),
    /// Creates or updates a field definition.
    UpsertFieldDef(AnyFieldDef),
    UpsertField(AnyField),
}

impl Command {
    pub(crate) fn execute(self, tx: &rusqlite::Transaction) -> rusqlite::Result<usize> {
        match self {
            Command::AddNote(note) => Command::add_note(tx, &note),
            Command::ArchiveNote(note) => Command::set_archived(tx, note.id(), true),
            Command::EditNote(note) => Command::edit_note(tx, &note),
            Command::UnarchiveNote(note) => Command::set_archived(tx, note.id(), false),
            Command::UpsertCollection(collection) => Command::upsert_collection(tx, &collection),
            Command::UpsertRecord(record) => Command::upsert_record(tx, &record),
            Command::UpsertFieldDef(field_def) => Command::upsert_field_def(tx, &field_def),
            Command::UpsertField(field) => Command::upsert_field(tx, &field),
        }
    }

    fn add_note(tx: &rusqlite::Transaction, note: &PileNote) -> rusqlite::Result<usize> {
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

    fn edit_note(tx: &rusqlite::Transaction, note: &PileNote) -> rusqlite::Result<usize> {
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
            _ => Err(rusqlite::Error::QueryReturnedNoRows),
        }
    }

    fn set_archived(
        tx: &rusqlite::Transaction,
        id: NoteId,
        archived: bool,
    ) -> rusqlite::Result<usize> {
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
            _ => Err(rusqlite::Error::QueryReturnedNoRows),
        }
    }

    fn upsert_collection(
        tx: &rusqlite::Transaction,
        collection: &Collection,
    ) -> rusqlite::Result<usize> {
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

    fn upsert_record(tx: &rusqlite::Transaction, record: &Record) -> rusqlite::Result<usize> {
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

    fn upsert_field_def(
        tx: &rusqlite::Transaction,
        field_def: &AnyFieldDef,
    ) -> rusqlite::Result<usize> {
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

    fn upsert_field(tx: &rusqlite::Transaction, field: &AnyField) -> rusqlite::Result<usize> {
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
}
