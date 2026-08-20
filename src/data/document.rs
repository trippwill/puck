// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tokio_rusqlite::rusqlite::{self};
use tokio_rusqlite::{Connection, OpenFlags};

use super::command::Command;
use super::migration::{self, CURRENT_VERSION, MINIMUM_COMPATIBLE_VERSION, MigrationError};
use super::version::SchemaVersion;
use crate::core::NoteError;
use crate::data::query::Query;

const APPLICATION_ID: i32 = i32::from_be_bytes(*b"PUCK");

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] tokio_rusqlite::Error),
    #[error("Invalid file: {0}")]
    InvalidFile(PathBuf),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Version mismatch for file {0}: expected at least {1}, found {2}")]
    VersionError(PathBuf, SchemaVersion, SchemaVersion),
    #[error("Unsupported version for file {0}: maximum supported is {1}, found {2}")]
    UnsupportedVersion(PathBuf, SchemaVersion, SchemaVersion),
    #[error("No migration path for file {0}: cannot upgrade from {1} to {2}")]
    MigrationUnavailable(PathBuf, SchemaVersion, SchemaVersion),
    #[error("Invalid embedded migration registry")]
    InvalidMigrationRegistry,
    #[error("Invalid persisted note: {0}")]
    InvalidNote(#[from] NoteError),
}

impl From<rusqlite::Error> for DocumentError {
    fn from(err: rusqlite::Error) -> Self {
        DocumentError::SqliteError(err.into())
    }
}

/// An open Puck document.
///
/// A `Document` owns an open connection to a validated Puck document and
/// records the Puck schema version of that document.
///
/// Instances are created through [`Document::open`] or [`Document::create`].
#[derive(Debug, Clone)]
pub struct Document {
    path: PathBuf,
    conn: Connection,
    version: SchemaVersion,
}

#[derive(Clone, Copy)]
enum ConnectMode {
    Create,
    Open,
}

struct DocumentHeader {
    application_id: i32,
    version: SchemaVersion,
}

impl Document {
    /// Creates and opens a Puck document.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot create or configure the document.
    pub async fn create(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;

        // SQLite will open the file and write its header. We needed this
        // to reserve the filename atomically.
        drop(file);

        match connect(&path, ConnectMode::Create).await {
            Ok(doc) => Ok(doc),
            Err(error) => {
                if let Err(cleanup_error) = std::fs::remove_file(&path) {
                    tracing::warn!(
                        path = ?path,
                        error = %cleanup_error,
                        "failed to remove incomplete document"
                    );
                }
                Err(error)
            }
        }
    }

    /// Opens an existing document.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot open or configure the document.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        connect(path, ConnectMode::Open).await
    }

    /// Executes commands in a single transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if any command fails. No commands are persisted in that case.
    pub async fn execute(&self, commands: Vec<Command>) -> Result<(), DocumentError> {
        if commands.is_empty() {
            return Ok(());
        }

        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;
                for command in commands {
                    command.execute(&tx)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(Into::into)
    }

    /// Runs a typed query.
    ///
    /// # Errors
    ///
    /// Returns an error if the query or persisted-data validation fails.
    pub async fn query<Q: Query>(&self, query: Q) -> Result<Q::Output, DocumentError> {
        self.conn.call_raw(|conn| query.run(conn)).await?
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn version(&self) -> SchemaVersion {
        self.version
    }
}

async fn prepare_connection(
    conn: &Connection,
    kind: ConnectMode,
    path: PathBuf,
) -> Result<DocumentHeader, DocumentError> {
    conn.call_raw(move |conn| {
        conn.busy_timeout(Duration::from_secs(1))?;
        conn.execute_batch(
            r"
            PRAGMA foreign_keys = ON;
            PRAGMA locking_mode = EXCLUSIVE;
            PRAGMA journal_mode = DELETE;
            PRAGMA synchronous = FULL;
            BEGIN EXCLUSIVE;
            COMMIT;
            ",
        )?;

        if let ConnectMode::Create = kind {
            let tx = conn.transaction()?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            migration::initialize(&tx).map_err(|error| migration_error(&path, error))?;
            tx.commit()?;
        }

        let application_id =
            conn.pragma_query_value(None, "application_id", |row| row.get::<_, i32>(0))?;
        let user_version =
            conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))?;

        Ok(DocumentHeader {
            application_id,
            version: SchemaVersion::from_i32(user_version),
        })
    })
    .await?
}

async fn connect(path: impl AsRef<Path>, kind: ConnectMode) -> Result<Document, DocumentError> {
    let path = path.as_ref().to_path_buf();
    let conn = match kind {
        ConnectMode::Create => Connection::open(&path).await?,
        ConnectMode::Open => {
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE).await?
        }
    };

    let header = prepare_connection(&conn, kind, path.clone()).await?;
    if let Err(error) = validate_header(&path, &header) {
        return Err(close_rejected(conn, &path, error).await);
    }

    let version = if let ConnectMode::Open = kind {
        match migrate_connection(&conn, &path, header.version).await {
            Ok(version) => version,
            Err(error) => return Err(close_rejected(conn, &path, error).await),
        }
    } else {
        header.version
    };

    Ok(Document {
        path,
        conn,
        version,
    })
}

async fn migrate_connection(
    conn: &Connection,
    path: &Path,
    from: SchemaVersion,
) -> Result<SchemaVersion, DocumentError> {
    let path = path.to_path_buf();
    conn.call_raw(move |conn| {
        migration::migrate(conn, from).map_err(|error| migration_error(&path, error))
    })
    .await?
}

fn migration_error(path: &Path, error: MigrationError) -> DocumentError {
    match error {
        MigrationError::InvalidRegistry => DocumentError::InvalidMigrationRegistry,
        MigrationError::Sqlite(error) => error.into(),
        MigrationError::UnregisteredVersion(version) => {
            DocumentError::MigrationUnavailable(path.to_path_buf(), version, CURRENT_VERSION)
        }
    }
}

async fn close_rejected(conn: Connection, path: &Path, error: DocumentError) -> DocumentError {
    if let Err(close_error) = conn.close().await {
        tracing::warn!(
            path = ?path,
            error = %close_error,
            "failed to close rejected document"
        );
    }
    error
}

fn validate_header(path: &Path, header: &DocumentHeader) -> Result<(), DocumentError> {
    if header.application_id != APPLICATION_ID {
        return Err(DocumentError::InvalidFile(path.to_path_buf()));
    }

    if header.version > CURRENT_VERSION {
        return Err(DocumentError::UnsupportedVersion(
            path.to_path_buf(),
            CURRENT_VERSION,
            header.version,
        ));
    }

    if header.version < MINIMUM_COMPATIBLE_VERSION {
        return Err(DocumentError::VersionError(
            path.to_path_buf(),
            MINIMUM_COMPATIBLE_VERSION,
            header.version,
        ));
    }

    Ok(())
}

impl Drop for Document {
    fn drop(&mut self) {
        tracing::trace!("Releasing document {:?}", self.path());
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use time::{Month, OffsetDateTime};
    use tokio_rusqlite::params;

    use super::*;
    use crate::core::prelude::*;
    use crate::data::query::{
        CollectionById,
        Collections,
        FieldByKey,
        FieldDefById,
        FieldDefs,
        FieldsByRecord,
        NoteById,
        NoteSummaries,
        RecordById,
        RecordsByCollection,
    };

    #[test]
    fn invalid_application_id_is_rejected() {
        let header = DocumentHeader {
            application_id: 0,
            version: CURRENT_VERSION,
        };

        assert!(matches!(
            validate_header(Path::new("invalid.puck"), &header),
            Err(DocumentError::InvalidFile(_))
        ));
    }

    #[test]
    fn future_version_is_rejected_as_unsupported() {
        let future_version = SchemaVersion::new(
            CURRENT_VERSION.major(),
            CURRENT_VERSION.minor(),
            CURRENT_VERSION
                .migration()
                .checked_add(1)
                .expect("test requires a future migration version"),
        );
        let header = DocumentHeader {
            application_id: APPLICATION_ID,
            version: future_version,
        };

        assert!(matches!(
            validate_header(Path::new("future.puck"), &header),
            Err(DocumentError::UnsupportedVersion(_, supported, found))
                if supported == CURRENT_VERSION && found == future_version
        ));
    }

    #[tokio::test]
    async fn new_document_sets_header() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        assert!(Document::open(&path).await.is_err());

        let document = Document::create(&path).await.unwrap();

        let (application_id, user_version) = document
            .conn
            .call(|conn| {
                let application_id =
                    conn.pragma_query_value(None, "application_id", |row| row.get::<_, i32>(0))?;
                let user_version =
                    conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))?;
                Ok::<_, tokio_rusqlite::rusqlite::Error>((application_id, user_version))
            })
            .await
            .unwrap();

        assert_eq!(application_id, APPLICATION_ID);
        assert_eq!(SchemaVersion::from_i32(user_version), CURRENT_VERSION);
        drop(document);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn current_document_reopens_without_rerunning_baseline() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let note = PileNote::create("Keep me");
        let id = note.id();
        document
            .execute(vec![Command::AddNote(note.clone())])
            .await
            .unwrap();
        drop(document);

        let document = Document::open(&path).await.unwrap();
        assert_eq!(document.version(), CURRENT_VERSION);
        assert_eq!(document.query(NoteById(id)).await.unwrap(), Some(note));

        let user_version = document
            .conn
            .call(|conn| conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0)))
            .await
            .unwrap();
        assert_eq!(SchemaVersion::from_i32(user_version), CURRENT_VERSION);

        drop(document);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn notes_round_trip_and_list_by_update_time() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let now = OffsetDateTime::now_utc();
        let older = PileNote::restore(
            NoteId::new(),
            String::from("Older\nsecond line"),
            u32::MAX,
            now - time::Duration::SECOND,
            now - time::Duration::SECOND,
        )
        .unwrap();
        let newer = PileNote::restore(NoteId::new(), String::from("Newer"), 1, now, now).unwrap();
        let older_id = older.id();
        let newer_id = newer.id();

        document
            .execute(vec![
                Command::AddNote(older.clone()),
                Command::AddNote(newer.clone()),
            ])
            .await
            .unwrap();

        let stored = document.query(NoteById(older_id)).await.unwrap().unwrap();
        assert_eq!(stored, older);
        assert_eq!(document.query(NoteById(NoteId::new())).await.unwrap(), None);

        let summaries = document.query(NoteSummaries).await.unwrap();
        assert_eq!(
            summaries
                .iter()
                .map(|summary| summary.id)
                .collect::<Vec<_>>(),
            [newer_id, older_id]
        );
        assert_eq!(summaries[1].preview, "Older");

        let row = document
            .conn
            .call(move |conn| {
                conn.query_row(
                    r"
                    SELECT id, body, revision, created_at, updated_at, archived
                    FROM notes
                    WHERE id = ?1
                    ",
                    [*older_id.as_uuid()],
                    |row| {
                        Ok((
                            row.get::<_, uuid::Uuid>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, u32>(2)?,
                            row.get::<_, OffsetDateTime>(3)?,
                            row.get::<_, OffsetDateTime>(4)?,
                            row.get::<_, i64>(5)?,
                        ))
                    },
                )
            })
            .await
            .unwrap();
        assert_eq!(row.0, *older_id.as_uuid());
        assert_eq!(row.1, older.body());
        assert_eq!(row.2, older.revision());
        assert_eq!(row.3, older.created_at());
        assert_eq!(row.4, older.updated_at());
        assert_eq!(row.5, 0);

        drop(document);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn structured_data_round_trips() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let collection = Collection::new("Values");
        let record = collection.new_record();
        let second_record = collection.new_record();
        let other_collection = Collection::new("Other");
        let other_record = other_collection.new_record();
        let text_def = Text::def("Text");
        let boolean_def = Boolean::def("Boolean");
        let integer_def = Integer::def("Integer");
        let date_def = Date::def("Date");
        let time_def = Time::def("Time");
        let timestamp_def = Timestamp::def("Timestamp");
        let date = time::Date::from_calendar_date(2026, Month::August, 20).unwrap();
        let time = time::Time::from_hms(19, 52, 54).unwrap();
        let timestamp = time::Timestamp::from_milliseconds(1_777_777_777_777).unwrap();
        let collection_id = collection.id();
        let record_id = record.id();
        let second_record_id = second_record.id();
        let other_collection_id = other_collection.id();
        let other_record_id = other_record.id();
        let text_id = text_def.id();
        let boolean_id = boolean_def.id();
        let integer_id = integer_def.id();
        let date_id = date_def.id();
        let time_id = time_def.id();
        let timestamp_id = timestamp_def.id();
        let text_field = record.new_field(&text_def, String::from("hello"));
        let boolean_field = record.new_field(&boolean_def, true);
        let integer_field = record.new_field(&integer_def, -42);
        let date_field = record.new_field(&date_def, date);
        let time_field = record.new_field(&time_def, time);
        let timestamp_field = record.new_field(&timestamp_def, timestamp);
        let other_text_field = other_record.new_field(&text_def, String::from("other"));
        let text_key = text_field.key();

        assert!(document.query(Collections).await.unwrap().is_empty());
        assert!(document.query(FieldDefs).await.unwrap().is_empty());
        assert!(
            document
                .query(RecordsByCollection(collection_id))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            document
                .query(FieldsByRecord(record_id))
                .await
                .unwrap()
                .is_empty()
        );

        document
            .execute(vec![
                Command::UpsertCollection(collection),
                Command::UpsertRecord(record),
                Command::UpsertRecord(second_record),
                Command::UpsertCollection(other_collection),
                Command::UpsertRecord(other_record),
                Command::UpsertFieldDef(AnyFieldDef::Text(text_def)),
                Command::UpsertFieldDef(AnyFieldDef::Boolean(boolean_def)),
                Command::UpsertFieldDef(AnyFieldDef::Integer(integer_def)),
                Command::UpsertFieldDef(AnyFieldDef::Date(date_def)),
                Command::UpsertFieldDef(AnyFieldDef::Time(time_def)),
                Command::UpsertFieldDef(AnyFieldDef::Timestamp(timestamp_def)),
                Command::UpsertField(AnyField::Text(text_field)),
                Command::UpsertField(AnyField::Boolean(boolean_field)),
                Command::UpsertField(AnyField::Integer(integer_field)),
                Command::UpsertField(AnyField::Date(date_field)),
                Command::UpsertField(AnyField::Time(time_field)),
                Command::UpsertField(AnyField::Timestamp(timestamp_field)),
                Command::UpsertField(AnyField::Text(other_text_field)),
            ])
            .await
            .unwrap();

        let mut expected_collection_ids =
            vec![collection_id.to_string(), other_collection_id.to_string()];
        expected_collection_ids.sort();
        assert_eq!(
            document
                .query(Collections)
                .await
                .unwrap()
                .into_iter()
                .map(|collection| collection.id().to_string())
                .collect::<Vec<_>>(),
            expected_collection_ids
        );

        let mut expected_record_ids = vec![record_id.to_string(), second_record_id.to_string()];
        expected_record_ids.sort();
        assert_eq!(
            document
                .query(RecordsByCollection(collection_id))
                .await
                .unwrap()
                .into_iter()
                .map(|record| record.id().to_string())
                .collect::<Vec<_>>(),
            expected_record_ids
        );
        assert_eq!(
            document
                .query(RecordsByCollection(other_collection_id))
                .await
                .unwrap()
                .into_iter()
                .map(|record| record.id())
                .collect::<Vec<_>>(),
            vec![other_record_id]
        );

        let mut expected_def_ids = vec![
            text_id.to_string(),
            boolean_id.to_string(),
            integer_id.to_string(),
            date_id.to_string(),
            time_id.to_string(),
            timestamp_id.to_string(),
        ];
        expected_def_ids.sort();
        assert_eq!(
            document
                .query(FieldDefs)
                .await
                .unwrap()
                .into_iter()
                .map(|def| def.id().to_string())
                .collect::<Vec<_>>(),
            expected_def_ids
        );

        let mut expected_field_def_ids = vec![
            text_id.to_string(),
            boolean_id.to_string(),
            integer_id.to_string(),
            date_id.to_string(),
            time_id.to_string(),
            timestamp_id.to_string(),
        ];
        expected_field_def_ids.sort();
        assert_eq!(
            document
                .query(FieldsByRecord(record_id))
                .await
                .unwrap()
                .into_iter()
                .map(|field| field.def_id().to_string())
                .collect::<Vec<_>>(),
            expected_field_def_ids
        );
        assert!(
            document
                .query(FieldsByRecord(second_record_id))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            document
                .query(FieldsByRecord(other_record_id))
                .await
                .unwrap()
                .into_iter()
                .map(|field| field.def_id())
                .collect::<Vec<_>>(),
            vec![text_id]
        );

        let stored_collection = document
            .query(CollectionById(collection_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored_collection.id(), collection_id);
        assert_eq!(stored_collection.name(), "Values");

        let stored_record = document
            .query(RecordById(record_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored_record.id(), record_id);
        assert_eq!(stored_record.collection_id(), collection_id);

        assert!(matches!(
            document.query(FieldDefById(text_id)).await.unwrap(),
            Some(AnyFieldDef::Text(def)) if def.name() == "Text"
        ));
        assert!(matches!(
            document.query(FieldDefById(boolean_id)).await.unwrap(),
            Some(AnyFieldDef::Boolean(def)) if def.name() == "Boolean"
        ));
        assert!(matches!(
            document.query(FieldDefById(integer_id)).await.unwrap(),
            Some(AnyFieldDef::Integer(def)) if def.name() == "Integer"
        ));
        assert!(matches!(
            document.query(FieldDefById(date_id)).await.unwrap(),
            Some(AnyFieldDef::Date(def)) if def.name() == "Date"
        ));
        assert!(matches!(
            document.query(FieldDefById(time_id)).await.unwrap(),
            Some(AnyFieldDef::Time(def)) if def.name() == "Time"
        ));
        assert!(matches!(
            document.query(FieldDefById(timestamp_id)).await.unwrap(),
            Some(AnyFieldDef::Timestamp(def)) if def.name() == "Timestamp"
        ));

        assert!(matches!(
            document.query(FieldByKey(text_key)).await.unwrap(),
            Some(AnyField::Text(field)) if field.val() == "hello"
        ));
        assert!(matches!(
            document
                .query(FieldByKey((record_id, boolean_id)))
                .await
                .unwrap(),
            Some(AnyField::Boolean(field)) if *field.val()
        ));
        assert!(matches!(
            document
                .query(FieldByKey((record_id, integer_id)))
                .await
                .unwrap(),
            Some(AnyField::Integer(field)) if *field.val() == -42
        ));
        assert!(matches!(
            document
                .query(FieldByKey((record_id, date_id)))
                .await
                .unwrap(),
            Some(AnyField::Date(field)) if *field.val() == date
        ));
        assert!(matches!(
            document
                .query(FieldByKey((record_id, time_id)))
                .await
                .unwrap(),
            Some(AnyField::Time(field)) if *field.val() == time
        ));
        assert!(matches!(
            document
                .query(FieldByKey((record_id, timestamp_id)))
                .await
                .unwrap(),
            Some(AnyField::Timestamp(field)) if *field.val() == timestamp
        ));

        assert_eq!(
            document
                .query(CollectionById(CollectionId::new()))
                .await
                .unwrap()
                .map(|collection| collection.id()),
            None
        );
        assert_eq!(
            document
                .query(RecordById(RecordId::new()))
                .await
                .unwrap()
                .map(|record| record.id()),
            None
        );
        assert!(
            document
                .query(FieldDefById(FieldDefId::new()))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            document
                .query(FieldByKey((record_id, FieldDefId::new())))
                .await
                .unwrap()
                .is_none()
        );

        document
            .conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE fields SET value = 2 WHERE record_id = ?1 AND field_def_id = ?2",
                    params![*record_id.as_uuid(), *boolean_id.as_uuid()],
                )?;
                conn.execute(
                    "UPDATE fields SET value = ?3 WHERE record_id = ?1 AND field_def_id = ?2",
                    params![*record_id.as_uuid(), *timestamp_id.as_uuid(), i64::MAX],
                )?;
                Ok::<(), rusqlite::Error>(())
            })
            .await
            .unwrap();
        assert!(
            document
                .query(FieldByKey((record_id, boolean_id)))
                .await
                .is_err()
        );
        assert!(
            document
                .query(FieldByKey((record_id, timestamp_id)))
                .await
                .is_err()
        );

        drop(document);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn commands_roll_back_as_one_transaction() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let note = PileNote::create("Duplicate");
        let id = note.id();

        assert!(
            document
                .execute(vec![Command::AddNote(note.clone()), Command::AddNote(note),])
                .await
                .is_err()
        );
        assert_eq!(document.query(NoteById(id)).await.unwrap(), None);

        drop(document);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn open_document_has_exclusive_access() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();

        assert!(Document::open(&path).await.is_err());

        drop(document);
        Document::open(&path).await.unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn invalid_persisted_note_is_rejected() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let id = uuid::Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        document
            .conn
            .call(move |conn| {
                conn.execute_batch("PRAGMA ignore_check_constraints = ON;")?;
                conn.execute(
                    r"
                    INSERT INTO notes (id, body, revision, created_at, updated_at, archived)
                    VALUES (?1, 'Invalid', ?2, ?3, ?3, 0)
                    ",
                    params![id, 0_u32, now],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .unwrap();

        assert!(matches!(
            document.query(NoteById(NoteId::restore(id))).await,
            Err(DocumentError::InvalidNote(NoteError::InvalidRevision))
        ));

        drop(document);
        std::fs::remove_file(path).unwrap();
    }
}
