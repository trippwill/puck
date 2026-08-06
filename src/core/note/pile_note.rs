use thiserror::Error;
use time::OffsetDateTime;

use crate::core::note::NoteId;

pub const MAX_PREVIEW_CHARS: usize = 72;

#[derive(Debug, Error)]
pub enum NoteError {
    #[error("Note body cannot be empty")]
    Empty,
    #[error("Note revision must be greater than zero")]
    InvalidRevision,
    #[error("Note revision counter overflow")]
    RevisionOverflow,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PileNote {
    id: NoteId,
    body: String,
    revision: u64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl PileNote {
    /// Creates a new note with the given body.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is empty or consists only of whitespace.
    pub fn create(body: impl Into<String>) -> Result<Self, NoteError> {
        let now = OffsetDateTime::now_utc();
        let body = validate_body(body.into())?;
        Ok(Self {
            id: NoteId::new(),
            body,
            revision: 1,
            created_at: now,
            updated_at: now,
        })
    }

    /// Restores a note from persisted data.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is empty, the revision is zero, or the timestamps are invalid (e.g., `updated_at` is before `created_at`).
    pub fn restore(
        id: NoteId,
        body: String,
        revision: u64,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Result<Self, NoteError> {
        let body = validate_body(body)?;
        if updated_at < created_at {
            return Err(NoteError::InvalidTimestamp);
        }
        match revision {
            0 => Err(NoteError::InvalidRevision),
            _ => Ok(Self {
                id,
                body,
                revision,
                created_at,
                updated_at,
            }),
        }
    }

    /// Edits the note with a new body.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is empty or consists only of whitespace, or if the revision counter overflows.
    pub fn edit(&self, body: impl Into<String>) -> Result<Self, NoteError> {
        let body = validate_body(body.into())?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(NoteError::RevisionOverflow)?;
        let updated_at = OffsetDateTime::now_utc();
        Ok(Self {
            id: self.id,
            body,
            revision,
            created_at: self.created_at,
            updated_at,
        })
    }

    #[must_use]
    pub fn id(&self) -> NoteId {
        self.id
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}

fn validate_body(body: String) -> Result<String, NoteError> {
    if body.trim().is_empty() {
        Err(NoteError::Empty)
    } else {
        Ok(body)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PileNoteSummary {
    pub id: NoteId,
    pub preview: String,
    pub revision: u64,
    pub updated_at: OffsetDateTime,
}

impl From<&PileNote> for PileNoteSummary {
    fn from(note: &PileNote) -> Self {
        let mut preview: String = note
            .body()
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(MAX_PREVIEW_CHARS)
            .collect();

        if note.body().chars().count() > MAX_PREVIEW_CHARS {
            preview.push('…');
        }

        Self {
            id: note.id(),
            preview,
            revision: note.revision(),
            updated_at: note.updated_at(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_body_should_return_error() {
        let cases = vec!["", "   ", "\n", "\t"];
        for case in cases {
            let result = PileNote::create(case);
            assert!(matches!(result, Err(NoteError::Empty)));
        }
    }

    #[test]
    fn edit_should_increment_revision() {
        let note = PileNote::create("Initial body").expect("valid note");
        let updated_note = note.edit("Updated body").expect("valid edit");
        assert_eq!(updated_note.revision(), note.revision() + 1);
        assert_eq!(updated_note.body(), "Updated body");
        assert!(updated_note.updated_at() > note.updated_at());
        assert_eq!(updated_note.created_at(), note.created_at());
        assert_eq!(updated_note.id(), note.id());
    }

    #[test]
    fn edit_revision_overflow_should_return_error() {
        let note = PileNote::create("Initial body").expect("valid note");
        let mut updated_note = note.clone();
        updated_note.revision = u64::MAX;
        let result = updated_note.edit("Updated body");
        assert!(matches!(result, Err(NoteError::RevisionOverflow)));
    }

    #[test]
    fn restore_revision_zero_should_return_error() {
        let body = "Some body".to_string();
        let created_at = OffsetDateTime::now_utc();
        let updated_at = created_at;
        let result = PileNote::restore(NoteId::new(), body, 0, created_at, updated_at);
        assert!(matches!(result, Err(NoteError::InvalidRevision)));
    }
}
