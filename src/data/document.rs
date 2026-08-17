use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tokio_rusqlite::{Connection, OpenFlags};

use super::version::SchemaVersion;

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
            PRAGMA journal_mode = DELETE;
            PRAGMA synchronous = FULL;
            ",
        )?;

        if let ConnectMode::Create = kind {
            conn.pragma_update(None, "application_id", APPLICATION_ID)?;
            conn.pragma_update(None, "user_version", i32::from(CURRENT_VERSION))?;
            // TODO: Create the necessary tables and schema for a new document here.
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
}
