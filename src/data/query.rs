// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Built-in document queries.

use tokio_rusqlite::OptionalExtension;
use tokio_rusqlite::rusqlite::types::Type as SqlType;
use tokio_rusqlite::rusqlite::{
    Connection as SyncConnection,
    Error as SqlError,
    Result as SqlResult,
    Row as SqlRow,
    params,
};

use super::adapter::StoredNote;
use super::document::DocumentError;
use super::query_trait::Query;
use crate::core::prelude::*;
use crate::data::SqlFieldType;

/// A field definition annotated for a selected collection.
#[derive(Debug, Clone)]
pub struct CollectionFieldDef {
    /// The active field definition.
    pub definition: AnyFieldDef,
    /// Whether an active record in the collection already uses the definition.
    pub used_in_collection: bool,
}

/// A compact record projection for collection lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSummary {
    /// The source record ID.
    pub id: RecordId,
    /// The record's explicit display label.
    pub label: String,
    /// The number of active fields on the record.
    pub field_count: u32,
    /// The note the record was structured from, if any.
    pub source_note_id: Option<NoteId>,
}

/// A field value paired with its definition name.
#[derive(Debug, Clone)]
pub struct NamedField {
    /// The field definition's display name.
    pub name: String,
    /// The typed field value.
    pub field: AnyField,
}

/// A record and its active named fields.
#[derive(Debug, Clone)]
pub struct RecordDetail {
    /// The record.
    pub record: Record,
    /// Active fields ordered by definition ID.
    pub fields: Vec<NamedField>,
}

/// A source note in either active or archived state.
#[derive(Debug, Clone)]
pub enum SourceNote {
    /// A note in the active pile.
    Pile(PileNote),
    /// A note in the archive.
    Archive(ArchiveNote),
}

/// A query for a pile note by ID.
///
/// Produces an `Option` containing [`PileNote`].
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
///
/// Produces an `Option` containing [`ArchiveNote`].
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
///
/// Produces a `Vec` of [`NoteSummary`] values.
pub struct NoteSummaries;
impl Query for NoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_notes(conn, false, false)?
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
///
/// Produces a `Vec` of [`NoteSummary`] values.
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
///
/// Produces a `Vec` of [`NoteSummary`] values.
pub struct ArchivedNoteSummaries;
impl Query for ArchivedNoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_notes(conn, true, false)?
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

/// A query for summaries of notes marked for deletion.
///
/// Produces a `Vec` of [`NoteSummary`] values.
pub struct DeletedNoteSummaries;
impl Query for DeletedNoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_notes(conn, true, true)?
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
///
/// Produces an `Option` containing [`Collection`].
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
///
/// Produces a `Vec` of [`Collection`] values.
pub struct Collections;
impl Query for Collections {
    type Output = Vec<Collection>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_collections(conn, false)
    }
}

/// A query for all collections marked for deletion ordered by ID.
///
/// Produces a `Vec` of [`Collection`] values.
pub struct DeletedCollections;
impl Query for DeletedCollections {
    type Output = Vec<Collection>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_collections(conn, true)
    }
}

/// A query for a record by ID.
///
/// Produces an `Option` containing [`Record`].
pub struct RecordById(pub RecordId);
impl Query for RecordById {
    type Output = Option<Record>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        conn.query_row(
            r"
            SELECT records.collection_id, records.label, records.source_note_id
            FROM records
            JOIN collections ON collections.id = records.collection_id
            WHERE records.id = ?1
                AND records.deleted = 0
                AND collections.deleted = 0
            ",
            [*self.0.as_uuid()],
            |row| {
                Record::restore(
                    self.0,
                    CollectionId::restore(row.get(0)?),
                    &row.get::<_, String>(1)?,
                    row.get::<_, Option<uuid::Uuid>>(2)?.map(NoteId::restore),
                )
                .map_err(|error| {
                    SqlError::FromSqlConversionFailure(1, SqlType::Text, Box::new(error))
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for records in a collection ordered by ID.
///
/// Produces a `Vec` of [`Record`] values.
pub struct RecordsByCollection(pub CollectionId);
impl Query for RecordsByCollection {
    type Output = Vec<Record>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_records(conn, self.0, false)
    }
}

/// A query for compact records in a collection ordered by ID.
///
/// Produces a `Vec` of [`RecordSummary`] values.
pub struct RecordSummariesByCollection(pub CollectionId);
impl Query for RecordSummariesByCollection {
    type Output = Vec<RecordSummary>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT
                records.id,
                records.label,
                records.source_note_id,
                count(field_defs.id)
            FROM records
            JOIN collections ON collections.id = records.collection_id
            LEFT JOIN fields
                ON fields.record_id = records.id
                AND fields.deleted = 0
            LEFT JOIN field_defs
                ON field_defs.id = fields.field_def_id
                AND field_defs.deleted = 0
            WHERE records.collection_id = ?1
                AND records.deleted = 0
                AND collections.deleted = 0
            GROUP BY records.id
            ORDER BY records.id
            ",
        )?;
        statement
            .query_map([*self.0.as_uuid()], |row| {
                Ok(RecordSummary {
                    id: RecordId::restore(row.get(0)?),
                    label: row.get(1)?,
                    source_note_id: row.get::<_, Option<uuid::Uuid>>(2)?.map(NoteId::restore),
                    field_count: row.get(3)?,
                })
            })?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

/// A query for records marked for deletion in an active collection.
///
/// Produces a `Vec` of [`Record`] values.
pub struct DeletedRecordsByCollection(pub CollectionId);
impl Query for DeletedRecordsByCollection {
    type Output = Vec<Record>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_records(conn, self.0, true)
    }
}

/// A query for a field definition by ID.
///
/// Produces an `Option` containing [`AnyFieldDef`].
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
///
/// Produces a `Vec` of [`AnyFieldDef`] values.
pub struct FieldDefs;
impl Query for FieldDefs {
    type Output = Vec<AnyFieldDef>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_field_defs(conn, false)
    }
}

/// A query for all active field definitions annotated for a collection.
///
/// Definitions already used by active records in the collection sort first.
pub struct FieldDefsForCollection(pub CollectionId);
impl Query for FieldDefsForCollection {
    type Output = Vec<CollectionFieldDef>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT
                field_defs.id,
                field_defs.name,
                field_defs.type,
                EXISTS (
                    SELECT 1
                    FROM fields
                    JOIN records ON records.id = fields.record_id
                    WHERE fields.field_def_id = field_defs.id
                        AND fields.deleted = 0
                        AND records.deleted = 0
                        AND records.collection_id = ?1
                ) AS used_in_collection
            FROM field_defs
            WHERE field_defs.deleted = 0
            ORDER BY used_in_collection DESC, field_defs.name, field_defs.id
            ",
        )?;
        statement
            .query_map([*self.0.as_uuid()], |row| {
                Ok(CollectionFieldDef {
                    definition: read_field_def(row)?,
                    used_in_collection: row.get(3)?,
                })
            })?
            .collect::<SqlResult<_>>()
            .map_err(Into::into)
    }
}

/// A query for all field definitions marked for deletion ordered by ID.
///
/// Produces a `Vec` of [`AnyFieldDef`] values.
pub struct DeletedFieldDefs;
impl Query for DeletedFieldDefs {
    type Output = Vec<AnyFieldDef>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_field_defs(conn, true)
    }
}

/// A query for a field by its record and definition IDs.
///
/// Produces an `Option` containing [`AnyField`].
pub struct FieldByKey(pub FieldKey);
impl Query for FieldByKey {
    type Output = Option<AnyField>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let FieldKey(record_id, def_id) = self.0;
        conn.query_row(
            r"
            SELECT fields.record_id, fields.field_def_id, fields.type, fields.value
            FROM fields
            JOIN records ON records.id = fields.record_id
            JOIN collections ON collections.id = records.collection_id
            JOIN field_defs ON field_defs.id = fields.field_def_id
            WHERE fields.record_id = ?1
                AND fields.field_def_id = ?2
                AND fields.deleted = 0
                AND records.deleted = 0
                AND collections.deleted = 0
                AND field_defs.deleted = 0
            ",
            params![*record_id.as_uuid(), *def_id.as_uuid()],
            read_field,
        )
        .optional()
        .map_err(Into::into)
    }
}

/// A query for fields on a record ordered by definition ID.
///
/// Produces a `Vec` of [`AnyField`] values.
pub struct FieldsByRecord(pub RecordId);
impl Query for FieldsByRecord {
    type Output = Vec<AnyField>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_fields(conn, self.0, false)
    }
}

/// A query for a record and its active named fields.
pub struct RecordDetailById(pub RecordId);
impl Query for RecordDetailById {
    type Output = Option<RecordDetail>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let Some(record) = RecordById(self.0).run(conn)? else {
            return Ok(None);
        };
        let mut statement = conn.prepare(
            r"
            SELECT
                fields.record_id,
                fields.field_def_id,
                fields.type,
                fields.value,
                field_defs.name
            FROM fields
            JOIN field_defs ON field_defs.id = fields.field_def_id
            WHERE fields.record_id = ?1
                AND fields.deleted = 0
                AND field_defs.deleted = 0
            ORDER BY fields.field_def_id
            ",
        )?;
        let fields = statement
            .query_map([*self.0.as_uuid()], |row| {
                Ok(NamedField {
                    field: read_field(row)?,
                    name: row.get(4)?,
                })
            })?
            .collect::<SqlResult<_>>()?;
        Ok(Some(RecordDetail { record, fields }))
    }
}

/// A query for an active or archived source note by ID.
pub struct SourceNoteById(pub NoteId);
impl Query for SourceNoteById {
    type Output = Option<SourceNote>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        let stored = conn
            .query_row(
                r"
                SELECT id, body, revision, created_at, updated_at, archived
                FROM notes
                WHERE id = ?1 AND deleted = 0
                ",
                [*self.0.as_uuid()],
                |row| Ok((StoredNote::read(row)?, row.get::<_, bool>(5)?)),
            )
            .optional()?;
        stored
            .map(|(stored, archived)| {
                if archived {
                    stored.into_archive_note().map(SourceNote::Archive)
                } else {
                    stored.into_note().map(SourceNote::Pile)
                }
            })
            .transpose()
            .map_err(Into::into)
    }
}

/// A query for fields marked for deletion on an active record.
///
/// Produces a `Vec` of [`AnyField`] values.
pub struct DeletedFieldsByRecord(pub RecordId);
impl Query for DeletedFieldsByRecord {
    type Output = Vec<AnyField>;

    fn run(self, conn: &SyncConnection) -> Result<Self::Output, DocumentError> {
        stored_fields(conn, self.0, true)
    }
}

fn stored_collections(
    conn: &SyncConnection,
    deleted: bool,
) -> Result<Vec<Collection>, DocumentError> {
    let mut statement =
        conn.prepare("SELECT id, name FROM collections WHERE deleted = ?1 ORDER BY id")?;
    statement
        .query_map([deleted], |row| {
            Ok(Collection::restore(
                CollectionId::restore(row.get(0)?),
                &row.get::<_, String>(1)?,
            ))
        })?
        .collect::<SqlResult<_>>()
        .map_err(Into::into)
}

fn stored_records(
    conn: &SyncConnection,
    collection_id: CollectionId,
    deleted: bool,
) -> Result<Vec<Record>, DocumentError> {
    let mut statement = conn.prepare(
        r"
        SELECT records.id, records.label, records.source_note_id
        FROM records
        JOIN collections ON collections.id = records.collection_id
        WHERE records.collection_id = ?1
            AND records.deleted = ?2
            AND collections.deleted = 0
        ORDER BY records.id
        ",
    )?;
    statement
        .query_map(params![*collection_id.as_uuid(), deleted], |row| {
            Record::restore(
                RecordId::restore(row.get(0)?),
                collection_id,
                &row.get::<_, String>(1)?,
                row.get::<_, Option<uuid::Uuid>>(2)?.map(NoteId::restore),
            )
            .map_err(|error| SqlError::FromSqlConversionFailure(1, SqlType::Text, Box::new(error)))
        })?
        .collect::<SqlResult<_>>()
        .map_err(Into::into)
}

fn stored_field_defs(
    conn: &SyncConnection,
    deleted: bool,
) -> Result<Vec<AnyFieldDef>, DocumentError> {
    let mut statement =
        conn.prepare("SELECT id, name, type FROM field_defs WHERE deleted = ?1 ORDER BY id")?;
    statement
        .query_map([deleted], read_field_def)?
        .collect::<SqlResult<_>>()
        .map_err(Into::into)
}

fn stored_fields(
    conn: &SyncConnection,
    record_id: RecordId,
    deleted: bool,
) -> Result<Vec<AnyField>, DocumentError> {
    let mut statement = conn.prepare(
        r"
        SELECT fields.record_id, fields.field_def_id, fields.type, fields.value
        FROM fields
        JOIN records ON records.id = fields.record_id
        JOIN collections ON collections.id = records.collection_id
        JOIN field_defs ON field_defs.id = fields.field_def_id
        WHERE fields.record_id = ?1
            AND fields.deleted = ?2
            AND records.deleted = 0
            AND collections.deleted = 0
            AND field_defs.deleted = 0
        ORDER BY fields.field_def_id
        ",
    )?;
    statement
        .query_map(params![*record_id.as_uuid(), deleted], read_field)?
        .collect::<SqlResult<_>>()
        .map_err(Into::into)
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

fn stored_notes(
    conn: &SyncConnection,
    archived: bool,
    deleted: bool,
) -> SqlResult<Vec<StoredNote>> {
    let mut statement = conn.prepare(
        r"
        SELECT id, body, revision, created_at, updated_at
        FROM notes
        WHERE archived = ?1 AND deleted = ?2
        ORDER BY updated_at DESC, id DESC
        ",
    )?;

    statement
        .query_map(params![archived, deleted], StoredNote::read)?
        .collect()
}
