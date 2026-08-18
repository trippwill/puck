use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use time::OffsetDateTime;
use tokio_rusqlite::rusqlite::{OptionalExtension, Row, params};
use tokio_rusqlite::{Connection, OpenFlags};

use super::adapter::prelude::*;
use super::version::SchemaVersion;
use crate::core::{NoteError, NoteId, NoteSummary, PileNote};
const APPLICATION_ID: i32 = i32::from_be_bytes(*b"PUCK");
const CURRENT_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 0);
const MINIMUM_COMPATIBLE_VERSION: SchemaVersion = SchemaVersion::new(0, 0, 0);

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
    #[error("Invalid persisted note: {0}")]
    InvalidNote(#[from] NoteError),
}

impl From<tokio_rusqlite::rusqlite::Error> for DocumentError {
    fn from(err: tokio_rusqlite::rusqlite::Error) -> Self {
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

struct StoredNote {
    id: uuid::Uuid,
    body: String,
    revision: SqlU64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl StoredNote {
    fn read(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            body: row.get("body")?,
            revision: row.get("revision")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn into_note(self) -> Result<PileNote, NoteError> {
        PileNote::restore(
            NoteId::restore(self.id),
            self.body,
            self.revision.0,
            self.created_at,
            self.updated_at,
        )
    }
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

    /// Adds a pile note to the document.
    ///
    /// # Errors
    ///
    /// Returns an error if the note cannot be represented or inserted.
    pub async fn add_note(&self, note: PileNote) -> Result<(), DocumentError> {
        let id = *note.id().as_uuid();
        let body = note.body().to_owned();
        let revision = SqlU64(note.revision());
        let created_at = note.created_at();
        let updated_at = note.updated_at();

        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;
                tx.execute(
                    r"
                    INSERT INTO notes (id, body, revision, created_at, updated_at, archived)
                    VALUES (?1, ?2, ?3, ?4, ?5, 0)
                    ",
                    params![id, body, revision, created_at, updated_at],
                )?;
                tx.commit()
            })
            .await?;

        Ok(())
    }

    /// Returns pile-note summaries ordered by most recent update.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails or persisted note data is invalid.
    pub async fn note_summaries(&self) -> Result<Vec<NoteSummary>, DocumentError> {
        let stored: Vec<StoredNote> = self
            .conn
            .call(|conn| {
                let mut statement = conn.prepare(
                    r"
                    SELECT id, body, revision, created_at, updated_at
                    FROM notes
                    WHERE archived = 0
                    ORDER BY updated_at DESC, id DESC
                    ",
                )?;
                statement.query_map([], StoredNote::read)?.collect()
            })
            .await?;

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

    /// Returns a pile note by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails or persisted note data is invalid.
    pub async fn note(&self, id: NoteId) -> Result<Option<PileNote>, DocumentError> {
        let id = *id.as_uuid();
        let stored = self
            .conn
            .call(move |conn| {
                conn.query_row(
                    r"
                    SELECT id, body, revision, created_at, updated_at
                    FROM notes
                    WHERE id = ?1 AND archived = 0
                    ",
                    [id],
                    StoredNote::read,
                )
                .optional()
            })
            .await?;

        stored
            .map(StoredNote::into_note)
            .transpose()
            .map_err(Into::into)
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
) -> Result<DocumentHeader, DocumentError> {
    conn.call(move |conn| {
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
            tx.pragma_update(None, "user_version", i32::from(CURRENT_VERSION))?;
            tx.execute_batch(
                r"
                CREATE TABLE notes (
                    id BLOB PRIMARY KEY NOT NULL
                        CHECK (typeof(id) = 'blob' AND length(id) = 16),
                    body TEXT NOT NULL,
                    revision BLOB NOT NULL
                        CHECK (
                            typeof(revision) = 'blob'
                            AND length(revision) = 8
                            AND revision != x'0000000000000000'
                        ),
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL CHECK (updated_at >= created_at),
                    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1))
                ) STRICT;
                ",
            )?;
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
    .await
    .map_err(DocumentError::from)
}

async fn connect(path: impl AsRef<Path>, kind: ConnectMode) -> Result<Document, DocumentError> {
    let path = path.as_ref().to_path_buf();
    let conn = match kind {
        ConnectMode::Create => Connection::open(&path).await?,
        ConnectMode::Open => {
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE).await?
        }
    };

    let header = prepare_connection(&conn, kind).await?;
    if let Err(error) = validate_header(&path, &header) {
        if let Err(close_error) = conn.close().await {
            tracing::warn!(
                path = ?path,
                error = %close_error,
                "failed to close rejected document"
            );
        }
        return Err(error);
    }

    Ok(Document {
        path,
        conn,
        version: header.version,
    })
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
        tracing::trace!("Closing document {:?}", self.path());
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

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
    async fn notes_round_trip_and_list_by_update_time() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        let document = Document::create(&path).await.unwrap();
        let now = OffsetDateTime::now_utc();
        let older = PileNote::restore(
            NoteId::new(),
            String::from("Older\nsecond line"),
            u64::MAX,
            now - time::Duration::SECOND,
            now - time::Duration::SECOND,
        )
        .unwrap();
        let newer = PileNote::restore(NoteId::new(), String::from("Newer"), 1, now, now).unwrap();
        let older_id = older.id();
        let newer_id = newer.id();

        document.add_note(older.clone()).await.unwrap();
        document.add_note(newer.clone()).await.unwrap();

        let stored = document.note(older_id).await.unwrap().unwrap();
        assert_eq!(stored, older);
        assert_eq!(document.note(NoteId::new()).await.unwrap(), None);

        let summaries = document.note_summaries().await.unwrap();
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
                            row.get::<_, Vec<u8>>(2)?,
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
        assert_eq!(row.2, older.revision().to_be_bytes());
        assert_eq!(row.3, older.created_at());
        assert_eq!(row.4, older.updated_at());
        assert_eq!(row.5, 0);

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
                    params![id, SqlU64(0), now],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .unwrap();

        assert!(matches!(
            document.note(NoteId::restore(id)).await,
            Err(DocumentError::InvalidNote(NoteError::InvalidRevision))
        ));

        drop(document);
        std::fs::remove_file(path).unwrap();
    }
}
