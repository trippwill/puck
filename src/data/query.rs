// SPDX-License-Identifier: MPL-2.0

use tokio_rusqlite::{OptionalExtension, rusqlite};

use super::adapter::prelude::*;
use super::document::DocumentError;
use crate::core::prelude::*;

pub mod prelude {
    pub use super::{ArchivedNoteById, ArchivedNoteSummaries, NoteById, NoteSummaries};
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
        stored_note(conn, self.0, false)?
            .map(StoredNote::into_note)
            .transpose()
            .map_err(Into::into)
    }
}

/// Returns an archived note by ID.
///
/// # Errors
///
/// Returns an error if the query fails or persisted note data is invalid.
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

/// Returns a list of note summaries for all non-archived notes.
///
/// # Errors
///
/// Returns an error if the query or persisted-data validation fails.
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

/// Returns a list of note summaries for all archived notes.
///
/// # Errors
///
/// Returns an error if the query or persisted-data validation fails.
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
