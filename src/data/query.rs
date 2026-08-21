// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use tokio_rusqlite::OptionalExtension;
use tokio_rusqlite::rusqlite::types::Type as SqlType;
use tokio_rusqlite::rusqlite::{
    Connection as SyncConnection,
    Error as SqlError,
    Result as SqlResult,
    Row as SqlRow,
    params,
};

use super::adapter::prelude::*;
use super::document::DocumentError;
use crate::core::prelude::*;
use crate::data::SqlFieldType;

pub mod prelude {
    pub use super::{
        ArchivedNoteById,
        ArchivedNoteSummaries,
        CollectionById,
        Collections,
        FieldByKey,
        FieldDefById,
        FieldDefs,
        FieldsByRecord,
        NoteById,
        NoteSearch,
        NoteSummaries,
        RecordById,
        RecordsByCollection,
    };
}

/// A query to be executed against the document.
pub trait Query: Send + 'static {
    type Output: Send + 'static;

    /// Executes the query against the given database connection.
    /// # Errors
    /// Returns an error if the query fails or persisted data is invalid.
    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError>;
}

/// A query for a pile note by ID.
#[derive(Debug, Clone)]
pub struct NoteById(pub NoteId);
impl Query for NoteById {
    type Output = Option<PileNote>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
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

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
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

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
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

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT id, body, revision, created_at, updated_at
            FROM notes
            WHERE archived = 0 AND deleted = 0 AND instr(body, ?1) > 0
            ORDER BY updated_at DESC, id DESC
            ",
        )?;
        let stored = statement
            .query_map([self.0], StoredNote::read)?
            .collect::<SqlResult<Vec<_>>>()?;

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

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
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

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT name FROM collections WHERE id = ?1 AND deleted = 0",
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

/// A query for all collections ordered by ID.
pub struct Collections;
impl Query for Collections {
    type Output = Vec<Collection>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement =
            conn.prepare("SELECT id, name FROM collections WHERE deleted = 0 ORDER BY id")?;
        statement
            .query_map([], |row| {
                Ok(Collection::restore(
                    CollectionId::restore(row.get(0)?),
                    &row.get::<_, String>(1)?,
                ))
            })?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

/// A query for a record by ID.
pub struct RecordById(pub RecordId);
impl Query for RecordById {
    type Output = Option<Record>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT collection_id FROM records WHERE id = ?1 AND deleted = 0",
            [*self.0.as_uuid()],
            |row| Ok(Record::restore(self.0, CollectionId::restore(row.get(0)?))),
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for records in a collection ordered by ID.
pub struct RecordsByCollection(pub CollectionId);
impl Query for RecordsByCollection {
    type Output = Vec<Record>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            "SELECT id FROM records WHERE collection_id = ?1 AND deleted = 0 ORDER BY id",
        )?;
        statement
            .query_map([*self.0.as_uuid()], |row| {
                Ok(Record::restore(RecordId::restore(row.get(0)?), self.0))
            })?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

/// A query for a field definition by ID.
pub struct FieldDefById(pub FieldDefId);
impl Query for FieldDefById {
    type Output = Option<AnyFieldDef>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            "SELECT id, name, type FROM field_defs WHERE id = ?1 AND deleted = 0",
            [*self.0.as_uuid()],
            read_field_def,
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for all field definitions ordered by ID.
pub struct FieldDefs;
impl Query for FieldDefs {
    type Output = Vec<AnyFieldDef>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement =
            conn.prepare("SELECT id, name, type FROM field_defs WHERE deleted = 0 ORDER BY id")?;
        statement
            .query_map([], read_field_def)?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

/// A query for a field by its record and definition IDs.
pub struct FieldByKey(pub (RecordId, FieldDefId));
impl Query for FieldByKey {
    type Output = Option<AnyField>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let (record_id, def_id) = self.0;
        conn.query_row(
            r"
            SELECT record_id, field_def_id, type, value
            FROM fields
            WHERE record_id = ?1 AND field_def_id = ?2 AND deleted = 0
            ",
            params![*record_id.as_uuid(), *def_id.as_uuid()],
            read_field,
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for fields on a record ordered by definition ID.
pub struct FieldsByRecord(pub RecordId);
impl Query for FieldsByRecord {
    type Output = Vec<AnyField>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT record_id, field_def_id, type, value
            FROM fields
            WHERE record_id = ?1 AND deleted = 0
            ORDER BY field_def_id
            ",
        )?;
        statement
            .query_map([*self.0.as_uuid()], read_field)?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

fn read_field_def(row: &SqlRow<'_>) -> SqlResult<AnyFieldDef> {
    let id = FieldDefId::restore(row.get(0)?);
    let name: String = row.get(1)?;
    let kind: SqlFieldType = row.get(2)?;
    Ok(match kind {
        SqlFieldType::Text => AnyFieldDef::Text(FieldDef::<Text>::restore(id, &name)),
        SqlFieldType::Boolean => AnyFieldDef::Boolean(FieldDef::<Boolean>::restore(id, &name)),
        SqlFieldType::Integer => AnyFieldDef::Integer(FieldDef::<Integer>::restore(id, &name)),
        SqlFieldType::Date => AnyFieldDef::Date(FieldDef::<Date>::restore(id, &name)),
        SqlFieldType::Time => AnyFieldDef::Time(FieldDef::<Time>::restore(id, &name)),
        SqlFieldType::Timestamp => {
            AnyFieldDef::Timestamp(FieldDef::<Timestamp>::restore(id, &name))
        }
    })
}

fn read_field(row: &SqlRow<'_>) -> SqlResult<AnyField> {
    let record_id = RecordId::restore(row.get(0)?);
    let def_id = FieldDefId::restore(row.get(1)?);
    let kind: SqlFieldType = row.get(2)?;
    Ok(match kind {
        SqlFieldType::Text => {
            AnyField::Text(Field::<Text>::restore(def_id, record_id, row.get(3)?))
        }
        SqlFieldType::Boolean => AnyField::Boolean(Field::<Boolean>::restore(
            def_id,
            record_id,
            match row.get(3)? {
                0_i64 => false,
                1 => true,
                value => {
                    return Err(SqlError::FromSqlConversionFailure(
                        3,
                        SqlType::Integer,
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
            AnyField::Integer(Field::<Integer>::restore(def_id, record_id, row.get(3)?))
        }
        SqlFieldType::Date => {
            AnyField::Date(Field::<Date>::restore(def_id, record_id, row.get(3)?))
        }
        SqlFieldType::Time => {
            AnyField::Time(Field::<Time>::restore(def_id, record_id, row.get(3)?))
        }
        SqlFieldType::Timestamp => AnyField::Timestamp(Field::<Timestamp>::restore(
            def_id,
            record_id,
            time::Timestamp::from_milliseconds(row.get(3)?).map_err(|error| {
                SqlError::FromSqlConversionFailure(3, SqlType::Integer, Box::new(error))
            })?,
        )),
    })
}

fn stored_note(conn: &SyncConnection, id: NoteId, archived: bool) -> SqlResult<Option<StoredNote>> {
    conn.query_row(
        r"
        SELECT id, body, revision, created_at, updated_at
        FROM notes
        WHERE id = ?1 AND archived = ?2 AND deleted = 0
        ",
        params![*id.as_uuid(), archived],
        StoredNote::read,
    )
    .optional()
}

fn stored_notes(conn: &SyncConnection, archived: bool) -> SqlResult<Vec<StoredNote>> {
    let mut statement = conn.prepare(
        r"
        SELECT id, body, revision, created_at, updated_at
        FROM notes
        WHERE archived = ?1 AND deleted = 0
        ORDER BY updated_at DESC, id DESC
        ",
    )?;

    statement.query_map([archived], StoredNote::read)?.collect()
}
