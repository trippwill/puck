// SPDX-License-Identifier: MPL-2.0

use time::OffsetDateTime;
use tokio_rusqlite::{Row, rusqlite};

use crate::core::{NoteError, NoteId, PileNote};

pub mod prelude {
    pub use super::StoredNote;
}

pub struct StoredNote {
    id: uuid::Uuid,
    body: String,
    revision: u32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl StoredNote {
    pub fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            body: row.get("body")?,
            revision: row.get("revision")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
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
}
