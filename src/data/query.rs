use tokio_rusqlite::{OptionalExtension, rusqlite};

use super::adapter::prelude::*;
use super::document::DocumentError;
use crate::core::prelude::*;

pub mod prelude {
    pub use super::{NoteById, NoteSummaries};
}

pub trait Query: Send + 'static {
    type Output: Send + 'static;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError>;
}

/// Returns a pile note by ID.
///
/// # Errors
///
/// Returns an error if the query fails or persisted note data is invalid.
#[derive(Debug, Clone)]
pub struct NoteById(pub NoteId);
impl Query for NoteById {
    type Output = Option<PileNote>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        let stored = conn
            .query_row(
                r"
                SELECT id, body, revision, created_at, updated_at
                FROM notes
                WHERE id = ?1 AND archived = 0
                ",
                [*self.0.as_uuid()],
                StoredNote::read,
            )
            .optional()?;

        stored
            .map(StoredNote::into_note)
            .transpose()
            .map_err(Into::into)
    }
}

/// Returns a list of note summaries for all non-archived notes.
///
/// # Errors
///
/// Returns an error if the query or persisted-data validation fails.
pub struct NoteSummaries;
impl Query for NoteSummaries {
    type Output = Vec<NoteSummary>;

    fn run(self, conn: &rusqlite::Connection) -> Result<Self::Output, DocumentError> {
        let mut statement = conn.prepare(
            r"
            SELECT id, body, revision, created_at, updated_at
            FROM notes
            WHERE archived = 0
            ORDER BY updated_at DESC, id DESC
            ",
        )?;

        let stored = statement
            .query_map([], StoredNote::read)?
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
