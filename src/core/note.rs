use thiserror::Error;
use time::OffsetDateTime;

use crate::core::uuidv7_id;

/// The maximum number of characters in a pile-note preview.
pub const MAX_PREVIEW_CHARS: usize = 72;

uuidv7_id!(NoteId);

/// An error creating, restoring, or editing a pile note.
#[derive(Debug, Error)]
pub enum NoteError {
    /// The note body is empty or whitespace.
    #[error("Note body cannot be empty")]
    Empty,

    /// A restored revision is zero.
    #[error("Note revision must be greater than zero")]
    InvalidRevision,

    /// Editing would overflow the revision counter.
    #[error("Note revision counter overflow")]
    RevisionOverflow,

    /// The updated timestamp precedes the creation timestamp.
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

/// An immutable revision of a free-form pile note.
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
        Ok(Self { id: NoteId::new(), body, revision: 1, created_at: now, updated_at: now })
    }

    /// Restores a note from persisted data.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is empty, the revision is zero, or the timestamps are invalid
    /// (e.g., `updated_at` is before `created_at`).
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
            _ => Ok(Self { id, body, revision, created_at, updated_at }),
        }
    }

    /// Edits the note with a new body.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is empty or consists only of whitespace, or if the revision
    /// counter overflows.
    pub fn edit(&self, body: impl Into<String>) -> Result<Self, NoteError> {
        let body = validate_body(body.into())?;
        let revision = self.revision.checked_add(1).ok_or(NoteError::RevisionOverflow)?;
        let updated_at = OffsetDateTime::now_utc();
        Ok(Self { id: self.id, body, revision, created_at: self.created_at, updated_at })
    }

    /// Returns the note ID.
    #[must_use]
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// Returns the full note body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns the revision number.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns when the note was created.
    #[must_use]
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Returns when this revision was created.
    #[must_use]
    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}

fn validate_body(body: String) -> Result<String, NoteError> {
    if body.trim().is_empty() { Err(NoteError::Empty) } else { Ok(body) }
}

/// A compact pile-note projection for lists and search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PileNoteSummary {
    /// The source note ID.
    pub id: NoteId,
    /// The first line, truncated to [`MAX_PREVIEW_CHARS`] characters.
    pub preview: String,
    /// The source note revision.
    pub revision: u64,
    /// The source note's update timestamp.
    pub updated_at: OffsetDateTime,
}

impl From<&PileNote> for PileNoteSummary {
    fn from(note: &PileNote) -> Self {
        let first_line = note.body().lines().next().unwrap_or_default();
        let mut preview: String = first_line.chars().take(MAX_PREVIEW_CHARS).collect();

        if first_line.chars().count() > MAX_PREVIEW_CHARS {
            preview.push('…');
        }

        Self { id: note.id(), preview, revision: note.revision(), updated_at: note.updated_at() }
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

    #[test]
    fn restore_validates_timestamps_and_preserves_data() {
        let id = NoteId::new();
        let created_at = OffsetDateTime::now_utc();
        let updated_at = created_at + time::Duration::SECOND;

        assert!(matches!(
            PileNote::restore(id, String::from("Body"), 2, updated_at, created_at),
            Err(NoteError::InvalidTimestamp)
        ));

        let note = PileNote::restore(id, String::from("Body"), 2, created_at, updated_at).unwrap();
        assert_eq!(note.id(), id);
        assert_eq!(note.body(), "Body");
        assert_eq!(note.revision(), 2);
        assert_eq!(note.created_at(), created_at);
        assert_eq!(note.updated_at(), updated_at);
    }

    #[test]
    fn summary_uses_and_truncates_only_the_first_line() {
        let short = PileNote::create(
            "Short title\nThis second line is deliberately much longer than the preview limit and \
             must not affect truncation",
        )
        .unwrap();
        assert_eq!(PileNoteSummary::from(&short).preview, "Short title");

        let body = "🦀".repeat(MAX_PREVIEW_CHARS + 1);
        let note = PileNote::create(body).unwrap();
        let summary = PileNoteSummary::from(&note);
        assert_eq!(summary.preview, format!("{}…", "🦀".repeat(MAX_PREVIEW_CHARS)));
        assert_eq!(summary.preview.chars().count(), MAX_PREVIEW_CHARS + 1);
    }
}
