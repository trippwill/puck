#![allow(dead_code)]

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio_rusqlite::{Connection, rusqlite::OpenFlags};

const APPLICATION_ID: i32 = i32::from_be_bytes(*b"PUCK");

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] tokio_rusqlite::Error),
    #[error("Invalid file: {0}")]
    InvalidFile(PathBuf),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<tokio_rusqlite::rusqlite::Error> for DocumentError {
    fn from(err: tokio_rusqlite::rusqlite::Error) -> Self {
        DocumentError::SqliteError(err.into())
    }
}

#[derive(Debug)]
pub enum Document {
    Closed(PathBuf),
    Open(DocHandle),
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

        match DocHandle::connect(&path, true).await {
            Ok(handle) => Ok(Self::Open(handle)),
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
        Ok(Self::Open(DocHandle::connect(path, false).await?))
    }
}

#[derive(Debug)]
pub struct DocHandle {
    path: PathBuf,
    conn: Connection,
}

impl DocHandle {
    async fn connect(path: impl AsRef<Path>, new: bool) -> Result<Self, DocumentError> {
        let path = path.as_ref().to_path_buf();
        let conn = match new {
            true => Connection::open(&path).await?,
            false => Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE).await?,
        };

        let application_id = conn
            .call(move |c| {
                c.busy_timeout(std::time::Duration::from_secs(1))?;
                c.execute_batch(
                    r"
                  PRAGMA foreign_keys = ON;
                  PRAGMA journal_mode = DELETE;
                  PRAGMA synchronous = FULL;
                  ",
                )?;

                if new {
                    c.pragma_update(None, "application_id", APPLICATION_ID)?;
                    // TODO: Create the necessary tables and schema for a new document here.
                }

                c.pragma_query_value(None, "application_id", |row| row.get::<_, i32>(0))
            })
            .await?;

        if application_id != APPLICATION_ID {
            conn.close().await?;
            return Err(DocumentError::InvalidFile(path));
        }

        Ok(Self { conn, path })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::{APPLICATION_ID, Document};

    #[tokio::test]
    async fn new_document_sets_application_id() {
        let path = std::env::temp_dir().join(format!("puck-{}.db", uuid::Uuid::now_v7()));
        assert!(Document::open(&path).await.is_err());

        let Document::Open(document) = Document::create(&path).await.unwrap() else {
            unreachable!();
        };

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
