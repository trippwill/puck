// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use time::Timestamp;
use tokio_rusqlite::{Row, rusqlite};

use crate::core::{ArchiveNote, NoteError, NoteId, PileNote};

pub(crate) struct StoredNote {
    id: uuid::Uuid,
    body: String,
    revision: u32,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl StoredNote {
    pub fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            body: row.get("body")?,
            revision: row.get("revision")?,
            created_at: timestamp(row, 3)?,
            updated_at: timestamp(row, 4)?,
        })
    }

    pub fn into_note(self) -> Result<PileNote, NoteError> {
        PileNote::restore(
            NoteId::restore(self.id),
            self.body,
            self.revision,
            self.created_at,
            self.updated_at,
        )
    }

    pub fn into_archive_note(self) -> Result<ArchiveNote, NoteError> {
        ArchiveNote::restore(
            NoteId::restore(self.id),
            self.body,
            self.revision,
            self.created_at,
            self.updated_at,
        )
    }
}

fn timestamp(row: &Row<'_>, column: usize) -> rusqlite::Result<Timestamp> {
    Timestamp::from_milliseconds(row.get(column)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
