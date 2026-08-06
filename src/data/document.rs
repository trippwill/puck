use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use tokio_rusqlite::{Connection, OpenFlags};

use super::version::Version;

const APPLICATION_ID: i32 = i32::from_be_bytes(*b"PUCK");
const CURRENT_VERSION: Version = Version { release: 0, schema: 0, migration: 0 };
const MINIMUM_COMPATIBLE_VERSION: Version = Version { release: 0, schema: 0, migration: 0 };

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] tokio_rusqlite::Error),
    #[error("Invalid file: {0}")]
    InvalidFile(PathBuf),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Version mismatch for file {0}: expected at least {1}, found {2}")]
    VersionError(PathBuf, Version, Version),
}

impl From<tokio_rusqlite::rusqlite::Error> for DocumentError {
    fn from(err: tokio_rusqlite::rusqlite::Error) -> Self {
        DocumentError::SqliteError(err.into())
    }
}
#[derive(Debug, Clone)]
pub struct Document {
    path: PathBuf,
    conn: Connection,
    version: Version,
}

enum ConnectKind {
    Create,
    Open,
}

struct ConnectResult(i32, Version);

impl Document {
    /// Creates and opens a Puck document.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot create or configure the document.
    pub async fn create(path: impl AsRef<Path>) -> Result<Self, super::DocumentError> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new().write(true).create_new(true).open(&path)?;

        // SQLite will open the file and write its header. We needed this
        // to reserve the filename atomically.
        drop(file);

        match Self::connect(&path, ConnectKind::Create).await {
            Ok(doc) => Ok(doc),
            Err(e) => {
                // If we fail to open the document, we should remove the file to avoid leaving a
                // corrupted or unusable file behind.
                let _ = std::fs::remove_file(&path);
                Err(e)
            }
        }
    }

    /// Opens an existing document.
    ///
    /// # Errors
    ///
    /// Returns an error when `SQLite` cannot open or configure the document.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        Self::connect(path, ConnectKind::Open).await
    }

    async fn connect(path: impl AsRef<Path>, ck: ConnectKind) -> Result<Self, DocumentError> {
        let path = path.as_ref().to_path_buf();
        let conn = match ck {
            ConnectKind::Create => Connection::open(&path).await?,
            ConnectKind::Open => {
                Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE).await?
            }
        };

        let ConnectResult(app_id, version) = conn
            .call(move |c| {
                c.busy_timeout(std::time::Duration::from_secs(1))?;
                c.execute_batch(
                    r"
                  PRAGMA foreign_keys = ON;
                  PRAGMA journal_mode = DELETE;
                  PRAGMA synchronous = FULL;
                  ",
                )?;

                if let ConnectKind::Create = ck {
                    c.pragma_update(None, "application_id", APPLICATION_ID)?;
                    c.pragma_update(None, "user_version", i32::from(CURRENT_VERSION))?;
                    // TODO: Create the necessary tables and schema for a new document here.
                }

                let app_id =
                    c.pragma_query_value(None, "application_id", |row| row.get::<_, i32>(0))?;
                let user_version =
                    c.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))?;
                let version = Version::from_i32(user_version);

                Ok(ConnectResult(app_id, version))
            })
            .await?;

        if app_id != APPLICATION_ID {
            conn.close().await?;
            return Err(DocumentError::InvalidFile(path));
        }

        if version > CURRENT_VERSION {
            conn.close().await?;
            return Err(DocumentError::InvalidFile(path));
        }

        if version < MINIMUM_COMPATIBLE_VERSION {
            conn.close().await?;
            return Err(DocumentError::VersionError(path, MINIMUM_COMPATIBLE_VERSION, version));
        }

        Ok(Self { path, conn, version })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        tracing::trace!("Closing document {:?}", self.path());
    }
}

#[cfg(test)]
mod tests {
    use super::{APPLICATION_ID, Document};

    #[tokio::test]
    async fn new_document_sets_application_id() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        assert!(Document::open(&path).await.is_err());

        let document = Document::create(&path).await.unwrap();

        let application_id = document
            .conn
            .call(|conn| {
                conn.pragma_query_value(None, "application_id", |row| row.get::<_, i32>(0))
            })
            .await
            .unwrap();

        assert_eq!(application_id, APPLICATION_ID);
        drop(document);
        std::fs::remove_file(path).unwrap();
    }
}
