// SPDX-License-Identifier: MPL-2.0

use rusqlite::params;
use tokio_rusqlite::rusqlite;

use crate::core::prelude::*;

/// A command to be executed on the document.
#[derive(Debug)]
pub enum Command {
    /// Adds a pile note to the document.
    AddNote(PileNote),
    /// Persists an edited pile note.
    EditNote(PileNote),
}

impl Command {
    pub(crate) fn execute(self, tx: &rusqlite::Transaction) -> rusqlite::Result<usize> {
        match self {
            Command::AddNote(note) => Command::add_note(tx, &note),
            Command::EditNote(note) => Command::edit_note(tx, &note),
        }
    }

    fn add_note(tx: &rusqlite::Transaction, note: &PileNote) -> rusqlite::Result<usize> {
        tx.execute(
            r"
            INSERT INTO notes (id, body, revision, created_at, updated_at, archived)
            VALUES (?1, ?2, ?3, ?4, ?5, 0)
            ",
            params![
                *note.id().as_uuid(),
                note.body().to_owned(),
                note.revision(),
                note.created_at(),
                note.updated_at()
            ],
        )
    }

    fn edit_note(tx: &rusqlite::Transaction, note: &PileNote) -> rusqlite::Result<usize> {
        let changed = tx.execute(
            r"
            UPDATE notes
            SET body = ?2, revision = ?3, updated_at = ?4
            WHERE id = ?1 AND archived = 0
            ",
            params![
                *note.id().as_uuid(),
                note.body(),
                note.revision(),
                note.updated_at()
            ],
        )?;

        match changed {
            1 => Ok(changed),
            _ => Err(rusqlite::Error::QueryReturnedNoRows),
        }
    }
}
