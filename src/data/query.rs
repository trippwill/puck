// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use tokio_rusqlite::{OptionalExtension, rusqlite};

use super::adapter::prelude::*;
use super::document::DocumentError;
use crate::core::prelude::*;
use crate::data::SqlFieldType;

pub mod prelude {
    pub use super::{
        ArchivedNoteById,
        ArchivedNoteSummaries,
        CollectionById,
        FieldByKey,
        FieldDefById,
        NoteById,
        NoteSearch,
        NoteSummaries,
        RecordById,
    };
}

/// A query to be executed against the document.
pub trait Query: Send + 'static {
    type Output: Send + 'static;

    /// Executes the query against the given database connection.
    /// # Errors
    /// Returns an error if the query fails or persisted data is invalid.
    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError>;
}

/// A query for a pile note by ID.
#[derive(Debug, Clone)]
pub struct NoteById(pub NoteId);
impl Query for NoteById {
    type Output = Option<PileNote>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        stored_note(conn, self.0, false)?
            .map(StoredNote::into_note)
            .transpose()
            .map_err(Into::into)
    }
}

/// A query for an archived note by ID.
#[derive(Debug, Clone)]
pub struct ArchivedNoteById(pub NoteId);
impl Query for ArchivedNoteById {
    type Output = Option<ArchiveNote>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        stored_note(conn, self.0, true)?
            .map(StoredNote::into_archive_note)
            .transpose()
            .map_err(Into::into)
    }
}

/// A query for a list of note summaries for all non-archived notes.
pub struct NoteSummaries;
impl Query for NoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        stored_notes(conn, false)?
            .into_iter()
            .map(|stored| {
                stored
                    .into_note()
                    .map(|note| NoteSummary::from(&note))
                    .map_err(Into::into)
            })
            .collect()
    }
}

/// A query for active note summaries whose bodies contain a literal string.
pub struct NoteSearch(pub String);
impl Query for NoteSearch {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT id, body, revision, created_at, updated_at
            FROM notes
            WHERE archived = 0 AND instr(body, ?1) > 0
            ORDER BY updated_at DESC, id DESC
            ",
        )?;
        let stored = statement
            .query_map([self.0], StoredNote::read)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        stored
            .into_iter()
            .map(|stored| {
                stored
                    .into_note()
                    .map(|note| NoteSummary::from(&note))
                    .map_err(Into::into)
            })
            .collect()
    }
}

/// A query for a list of note summaries for all archived notes.
pub struct ArchivedNoteSummaries;
impl Query for ArchivedNoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        stored_notes(conn, true)?
            .into_iter()
            .map(|stored| {
                stored
                    .into_archive_note()
                    .map(|note| NoteSummary::from(&note))
                    .map_err(Into::into)
            })
            .collect()
    }
}

/// A query for a collection by ID.
pub struct CollectionById(pub CollectionId);
impl Query for CollectionById {
    type Output = Option<Collection>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT name FROM collections WHERE id = ?1",
            [*self.0.as_uuid()],
            |row| {
                let name: String = row.get(0)?;
                Ok(Collection::restore(self.0, &name))
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for a record by ID.
pub struct RecordById(pub RecordId);
impl Query for RecordById {
    type Output = Option<Record>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT collection_id FROM records WHERE id = ?1",
            [*self.0.as_uuid()],
            |row| Ok(Record::restore(self.0, CollectionId::restore(row.get(0)?))),
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for a field definition by ID.
pub struct FieldDefById(pub FieldDefId);
impl Query for FieldDefById {
    type Output = Option<AnyFieldDef>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT name, type FROM field_defs WHERE id = ?1",
            [*self.0.as_uuid()],
            |row| {
                let name: String = row.get(0)?;
                let kind: SqlFieldType = row.get(1)?;
                Ok(match kind {
                    SqlFieldType::Text => {
                        AnyFieldDef::Text(FieldDef::<Text>::restore(self.0, &name))
                    }
                    SqlFieldType::Boolean => {
                        AnyFieldDef::Boolean(FieldDef::<Boolean>::restore(self.0, &name))
                    }
                    SqlFieldType::Integer => {
                        AnyFieldDef::Integer(FieldDef::<Integer>::restore(self.0, &name))
                    }
                    SqlFieldType::Date => {
                        AnyFieldDef::Date(FieldDef::<Date>::restore(self.0, &name))
                    }
                    SqlFieldType::Time => {
                        AnyFieldDef::Time(FieldDef::<Time>::restore(self.0, &name))
                    }
                    SqlFieldType::Timestamp => {
                        AnyFieldDef::Timestamp(FieldDef::<Timestamp>::restore(self.0, &name))
                    }
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for a field by its record and definition IDs.
pub struct FieldByKey(pub (RecordId, FieldDefId));
impl Query for FieldByKey {
    type Output = Option<AnyField>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        let (record_id, def_id) = self.0;
        conn.query_row(
            "SELECT type, value FROM fields WHERE record_id = ?1 AND field_def_id = ?2",
            rusqlite::params![*record_id.as_uuid(), *def_id.as_uuid()],
            |row| {
                let kind: SqlFieldType = row.get(0)?;
                Ok(match kind {
                    SqlFieldType::Text => {
                        AnyField::Text(Field::<Text>::restore(def_id, record_id, row.get(1)?))
                    }
                    SqlFieldType::Boolean => AnyField::Boolean(Field::<Boolean>::restore(
                        def_id,
                        record_id,
                        match row.get(1)? {
                            0_i64 => false,
                            1 => true,
                            value => {
                                return Err(rusqlite::Error::FromSqlConversionFailure(
                                    1,
                                    rusqlite::types::Type::Integer,
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        format!("invalid boolean value: {value}"),
                                    )
                                    .into(),
                                ));
                            }
                        },
                    )),
                    SqlFieldType::Integer => {
                        AnyField::Integer(Field::<Integer>::restore(def_id, record_id, row.get(1)?))
                    }
                    SqlFieldType::Date => {
                        AnyField::Date(Field::<Date>::restore(def_id, record_id, row.get(1)?))
                    }
                    SqlFieldType::Time => {
                        AnyField::Time(Field::<Time>::restore(def_id, record_id, row.get(1)?))
                    }
                    SqlFieldType::Timestamp => AnyField::Timestamp(Field::<Timestamp>::restore(
                        def_id,
                        record_id,
                        time::Timestamp::from_milliseconds(row.get(1)?).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                    )),
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

fn stored_note(
    conn: &rusqlite::Connection,
    id: NoteId,
    archived: bool,
) -> rusqlite::Result<Option<StoredNote>> {
    conn.query_row(
        r"
        SELECT id, body, revision, created_at, updated_at
        FROM notes
        WHERE id = ?1 AND archived = ?2
        ",
        rusqlite::params![*id.as_uuid(), archived],
        StoredNote::read,
    )
    .optional()
}

fn stored_notes(conn: &rusqlite::Connection, archived: bool) -> rusqlite::Result<Vec<StoredNote>> {
    let mut statement = conn.prepare(
        r"
        SELECT id, body, revision, created_at, updated_at
        FROM notes
        WHERE archived = ?1
        ORDER BY updated_at DESC, id DESC
        ",
    )?;

    statement.query_map([archived], StoredNote::read)?.collect()
}
